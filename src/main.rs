use authority::{AuthorityProxy, PolkitError, Subject};
use dbus::AuthenticationAgent;
use eyre::{Result, WrapErr, ensure};
use futures_lite::StreamExt;
use gtk4::glib::{clone, spawn_future_local};
use relm4::RelmApp;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tracing::level_filters::LevelFilter;
use zbus::zvariant::Value;

use zbus::conn;

use crate::config::SystemConfig;
use crate::events::{AuthenticationAgentEvent, AuthenticationUserEvent};
use crate::ui::App;

mod authority;
mod config;
mod constants;
mod dbus;
mod events;
mod ui;

use gettextrs::{bindtextdomain, textdomain};

/// Upper bound for a single RegisterAuthenticationAgent attempt. polkitd can
/// acquire its well-known name before it is ready to answer method calls, and
/// a call landing in that window can hang indefinitely.
const REREGISTER_TIMEOUT: Duration = Duration::from_secs(10);
/// Initial delay before the first re-registration retry. The common failure
/// mode (D-Bus proxy destination-name cache race after an owner change)
/// resolves almost immediately, so start fast; a slow-recovery failure mode
/// (polkitd stuck in initialization) is rare but real, hence plain
/// exponential backoff (doubled after each failed attempt) up to the attempt
/// limit: 0.25s, 0.5s, 1s, ... 128s, ~4.3min total.
const REREGISTER_RETRY_DELAY_INITIAL: Duration = Duration::from_millis(250);
/// Maximum attempts per owner-change event before giving up and waiting for
/// the next event.
const REREGISTER_MAX_ATTEMPTS: usize = 10;

/// Result of racing a re-registration attempt against its timeout.
enum ReregisterOutcome {
    Done(Result<(), PolkitError>),
    TimedOut,
}

fn setup_tracing() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
                .add_directive("[start_object_server]=debug".parse()?),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing()?;

    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");

    let locale_path = std::env::var("SOTERIA_LOCALEDIR")
        .unwrap_or_else(|_| std::env!("SOTERIA_DEFAULT_LOCALE_DIR").into());

    bindtextdomain("soteria", &locale_path)?;
    textdomain("soteria")?;

    let config_path = std::env::var("XDG_CONFIG_HOME")
        .or(std::env::var("HOME").map(|e| e + "/.config"))
        .context("Could not resolve configuration path")?;
    let css_path = format!("{config_path}/soteria/style.css");
    let path = Path::new(&css_path);

    let config: SystemConfig = SystemConfig::from_file()?;

    let agent_helper = Path::new(config.get_helper_path());
    let agent_socket = Path::new(config.get_socket_path());

    ensure!(
        agent_helper.exists() || agent_socket.exists(),
        "Helper located at {} and socket located at {} do not exist.",
        &agent_helper.display(),
        &agent_socket.display()
    );

    if agent_socket.exists() {
        tracing::info!("using agent socket at {}", &agent_socket.display());
    } else {
        tracing::info!("using agent helper at {}", &agent_helper.display());
    }

    let (agent_sender, agent_receiver) = channel::<AuthenticationAgentEvent>(32);
    let (user_sender, user_receiver) = channel::<AuthenticationUserEvent>(32);

    let locale = gtk4::glib::language_names()[0].as_str().to_string();
    tracing::info!("Registering authentication agent with locale: {}", locale);
    let subject_kind = "unix-session".to_string();

    let subject_details = HashMap::from([(
        "session-id".to_string(),
        Value::new(
            std::env::var("XDG_SESSION_ID")
                .context("Could not get XDG session id, make sure that it is set and try again.")?,
        ),
    )]);
    let subject = Subject::new(subject_kind, subject_details);

    let agent = AuthenticationAgent::new(agent_sender, user_receiver, config.clone());
    let connection = conn::Builder::system()?
        .serve_at(constants::SELF_OBJECT_PATH, agent)?
        .build()
        .await?;

    let proxy = AuthorityProxy::new(&connection).await?;
    proxy
        .register_authentication_agent(&subject, &locale, constants::SELF_OBJECT_PATH)
        .await?;

    tracing::info!("Registered as authentication agent.");

    let mut owner_change_signal = proxy
        .inner()
        .receive_owner_changed()
        .await
        .context("Could not get the signal for owner change?")?;
    spawn_future_local(clone!(async move {
        while let Some(result) = owner_change_signal.next().await {
            // if we can get a UniqueName out of the stream, then polkit has restarted for some reason & we should reregister
            if result.is_some() {
                tracing::info!(
                    "Polkit's owner has changed, assuming the process restarted and reregistering."
                );
                // The registration call can hang forever when it lands in the
                // window where polkitd already owns its well-known name but has
                // not finished initializing (observed in the wild: one hung
                // call permanently silenced the listener, and every later
                // polkitd restart went unhandled until the agent was manually
                // restarted). Race each attempt against a timeout and retry
                // with a delay so a single bad window cannot disarm the agent.
                // Plain exponential backoff, reset per owner-change event.
                let mut retry_delay = REREGISTER_RETRY_DELAY_INITIAL;
                for attempt in 1..=REREGISTER_MAX_ATTEMPTS {
                    let call = proxy.register_authentication_agent(
                        &subject,
                        &locale,
                        constants::SELF_OBJECT_PATH,
                    );
                    let timeout = gtk4::glib::timeout_future(REREGISTER_TIMEOUT);
                    let outcome = futures_lite::future::race(
                        async { ReregisterOutcome::Done(call.await) },
                        async {
                            timeout.await;
                            ReregisterOutcome::TimedOut
                        },
                    )
                    .await;
                    match outcome {
                        ReregisterOutcome::Done(Ok(())) => {
                            tracing::info!(attempt, "Reregistered as authentication agent.");
                            break;
                        }
                        // A previous timed-out attempt may have reached polkitd
                        // after we stopped waiting; polkitd answers a duplicate
                        // registration with "already exists", which means the
                        // agent is registered and we are done.
                        ReregisterOutcome::Done(Err(err))
                            if err.to_string().contains("already exists") =>
                        {
                            tracing::info!(attempt, "Agent was already registered.");
                            break;
                        }
                        ReregisterOutcome::Done(Err(err)) => {
                            tracing::warn!(attempt, %err, "Failed to reregister.");
                        }
                        ReregisterOutcome::TimedOut => {
                            tracing::warn!(attempt, "Reregistration attempt timed out.");
                        }
                    }
                    if attempt < REREGISTER_MAX_ATTEMPTS {
                        gtk4::glib::timeout_future(retry_delay).await;
                        retry_delay = retry_delay.saturating_mul(2);
                    } else {
                        // We are a polkit authentication agent: without a
                        // successful registration we cannot answer any auth
                        // request. Exit so the service manager restarts us
                        // with a clean registration instead of lingering as
                        // a silent listener until the next polkit restart.
                        tracing::error!(
                            attempt,
                            "Reregistration failed after {} attempts; exiting.",
                            REREGISTER_MAX_ATTEMPTS
                        );
                        std::process::exit(1);
                    }
                }
            }
        }
    }));

    let app = RelmApp::new("gay.vaskel.soteria");
    if path.is_file() {
        tracing::info!("loading css stylesheet from {}", css_path);
        relm4::set_global_css_from_file(path)
            .context("Could not load CSS stylesheet for some reason")?;
    }
    app.run_async::<App>((user_sender, agent_receiver));

    Ok(())
}
