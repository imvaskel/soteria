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
const REGISTER_TIMEOUT: Duration = Duration::from_secs(10);
/// Initial delay before the first registration retry. The common failure
/// mode (D-Bus proxy destination-name cache race after an owner change)
/// resolves almost immediately, so start fast; a slow-recovery failure mode
/// (polkitd stuck in initialization) is rare but real, hence plain
/// exponential backoff (doubled after each failed attempt) up to the attempt
/// limit: 0.25s, 0.5s, 1s, ... 128s, ~4.3min total.
const REGISTER_RETRY_DELAY_INITIAL: Duration = Duration::from_millis(250);
/// Maximum registration attempts (startup and re-registration alike) before
/// giving up and exiting.
const REGISTER_MAX_ATTEMPTS: usize = 10;

/// Timer backend for the registration helper. The two call sites run on
/// different executors, and each must use its own executor's timer:
/// main() runs on the tokio runtime before the GTK main loop starts, where
/// glib::timeout_future panics on first poll (its SourceFuture asserts
/// that the polling thread owns the MainContext); the re-registration
/// future runs on the glib main context via spawn_future_local, where a
/// tokio timer would only work by silently attaching to whatever runtime
/// happens to be entered on the main thread (relm4 enters its own internal
/// runtime around the GTK main loop) — an implementation detail this code
/// should not depend on — so it uses the glib timer instead.
enum RegisterTimer {
    Tokio,
    Glib,
}

impl RegisterTimer {
    async fn sleep(&self, dur: Duration) {
        match self {
            RegisterTimer::Tokio => tokio::time::sleep(dur).await,
            RegisterTimer::Glib => gtk4::glib::timeout_future(dur).await,
        }
    }
}

/// Result of racing a registration attempt against its timeout.
enum RegisterOutcome {
    Done(Result<(), PolkitError>),
    TimedOut,
}

/// Register this authentication agent with polkitd, racing each attempt
/// against a timeout and retrying with exponential backoff. Shared by the
/// startup registration and the re-registration after a polkitd owner
/// change: both call sites race the same window where polkitd already owns
/// its well-known name but has not finished initializing, and a call landing
/// there can hang indefinitely (observed in the wild: one hung
/// re-registration permanently silenced the listener, and every later
/// polkitd restart went unhandled until the agent was manually restarted).
/// A previous timed-out attempt may still reach polkitd after we stopped
/// waiting; polkitd answers a duplicate registration with "already exists",
/// which therefore counts as success.
async fn register_with_retry(
    timer: RegisterTimer,
    proxy: &AuthorityProxy<'static>,
    subject: &Subject<'_>,
    locale: &str,
) {
    let mut retry_delay = REGISTER_RETRY_DELAY_INITIAL;
    for attempt in 1..=REGISTER_MAX_ATTEMPTS {
        let call =
            proxy.register_authentication_agent(subject, locale, constants::SELF_OBJECT_PATH);
        let outcome =
            futures_lite::future::race(async { RegisterOutcome::Done(call.await) }, async {
                timer.sleep(REGISTER_TIMEOUT).await;
                RegisterOutcome::TimedOut
            })
            .await;
        match outcome {
            RegisterOutcome::Done(Ok(())) => {
                tracing::info!(attempt, "Registered as authentication agent.");
                return;
            }
            // A previous timed-out attempt may have reached polkitd after we
            // stopped waiting; polkitd answers a duplicate registration with
            // "already exists", which means the agent is registered and we
            // are done.
            RegisterOutcome::Done(Err(err)) if err.to_string().contains("already exists") => {
                tracing::info!(attempt, "Agent was already registered.");
                return;
            }
            RegisterOutcome::Done(Err(err)) => {
                tracing::warn!(attempt, %err, "Failed to register.");
            }
            RegisterOutcome::TimedOut => {
                tracing::warn!(attempt, "Registration attempt timed out.");
            }
        }
        if attempt < REGISTER_MAX_ATTEMPTS {
            timer.sleep(retry_delay).await;
            retry_delay = retry_delay.saturating_mul(2);
        }
    }
    // We are a polkit authentication agent: without a successful
    // registration we cannot answer any auth request. Exit so the service
    // manager restarts us with a clean registration instead of lingering as
    // a process that cannot do its job.
    tracing::error!(
        attempt = REGISTER_MAX_ATTEMPTS,
        "Registration failed after {} attempts; exiting.",
        REGISTER_MAX_ATTEMPTS
    );
    std::process::exit(1);
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
    // Same exposure as the re-registration below: if soteria starts while
    // polkitd is mid-restart, the call can hang in the same window (or fail
    // while logind/polkitd state is still settling) and the agent never
    // comes up.
    register_with_retry(RegisterTimer::Tokio, &proxy, &subject, &locale).await;

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
                register_with_retry(RegisterTimer::Glib, &proxy, &subject, &locale).await;
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
