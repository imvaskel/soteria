use authority::{AuthorityProxy, Subject};
use dbus::AuthenticationAgent;
use eyre::{Result, WrapErr, ensure};
use relm4::RelmApp;
use std::collections::HashMap;
use std::path::Path;
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

    let locale_path = std::env::var("SOTERIA_LOCALEDIR").unwrap_or_else(|_| "/usr/share/locale".to_string());

    bindtextdomain("soteria", &locale_path)?;
    textdomain("soteria")?;

    let config_path = std::env::var("XDG_CONFIG_HOME")
        .or(std::env::var("HOME").map(|e| e + "/.config"))
        .context("Could not resolve configuration path")?;
    let css_path = format!("{config_path}/soteria/style.css");
    let path = Path::new(&css_path);

    let config: SystemConfig = SystemConfig::from_file()?;

    ensure!(
        Path::new(config.get_helper_path()).exists(),
        "Authentication helper located at {} does not exist.",
        config.get_helper_path()
    );
    tracing::info!(
        "using authentication helper located at {}",
        config.get_helper_path()
    );

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

    tracing::info!("Registered as authentication provider.");

    let app = RelmApp::new("gay.vaskel.soteria");
    if path.is_file() {
        tracing::info!("loading css stylesheet from {}", css_path);
        relm4::set_global_css_from_file(path)
            .context("Could not load CSS stylesheet for some reason")?;
    }
    app.run_async::<App>((user_sender, agent_receiver));

    Ok(())
}
