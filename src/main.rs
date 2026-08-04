use authority::{AuthorityProxy, Subject};
use dbus::AuthenticationAgent;
use eyre::{ContextCompat, Result, WrapErr, ensure};
use futures_lite::StreamExt;
use gtk4::glib::{clone, spawn_future_local};
use relm4::RelmApp;
use std::collections::HashMap;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{channel, unbounded_channel};
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
mod theme;
mod ui;

use gettextrs::{bindtextdomain, textdomain};

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

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(command) = theme::command_from_args(&args)? {
        return theme::run_command(command).await;
    }

    gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");

    let locale_path = std::env::var("SOTERIA_LOCALEDIR")
        .unwrap_or_else(|_| std::env!("SOTERIA_DEFAULT_LOCALE_DIR").into());

    bindtextdomain("soteria", &locale_path)?;
    textdomain("soteria")?;

    let config: SystemConfig = SystemConfig::from_file()?;
    let css_path = config
        .get_style_path()
        .map(ToOwned::to_owned)
        .map(Ok)
        .unwrap_or_else(theme::default_style_path)?;

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
                proxy
                    .register_authentication_agent(&subject, &locale, constants::SELF_OBJECT_PATH)
                    .await
                    .context("Somehow failed to reregister ourselves?")
                    .unwrap();
                tracing::info!("Reregistered as authentication agent.")
            }
        }
    }));

    let (theme_sender, mut theme_receiver) = unbounded_channel();
    let current_style_path = Arc::new(Mutex::new(css_path.clone()));
    let theme_control = theme::ThemeControl::new(current_style_path.clone(), theme_sender);
    let _theme_connection = conn::Builder::session()?
        .name(constants::THEME_BUS_NAME)?
        .serve_at(constants::THEME_OBJECT_PATH, theme_control)?
        .build()
        .await?;

    let app = RelmApp::new("gay.vaskel.soteria");
    let css_provider = gtk4::CssProvider::new();
    let display = gtk4::gdk::Display::default().context("Could not access GTK display")?;
    gtk4::style_context_add_provider_for_display(
        &display,
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    if css_path.is_file() {
        tracing::info!("loading css stylesheet from {}", css_path.display());
        css_provider.load_from_path(&css_path);
    }
    spawn_future_local(clone!(
        #[strong]
        css_provider,
        #[strong]
        current_style_path,
        async move {
            while let Some(request) = theme_receiver.recv().await {
                let path = match request {
                    theme::ThemeRequest::Reload => current_style_path
                        .lock()
                        .expect("theme path lock is poisoned")
                        .clone(),
                    theme::ThemeRequest::Load(path) => {
                        *current_style_path
                            .lock()
                            .expect("theme path lock is poisoned") = path.clone();
                        path
                    }
                };
                tracing::info!("reloading css stylesheet from {}", path.display());
                css_provider.load_from_path(path);
            }
        }
    ));
    app.run_async::<App>((user_sender, agent_receiver));

    Ok(())
}
