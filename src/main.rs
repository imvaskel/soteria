use authority::{AuthorityProxy, Subject};
use dbus::AuthenticationAgent;
use eyre::{OptionExt, Result, WrapErr, ensure};
use relm4::RelmApp;
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::broadcast::channel;
use tracing::level_filters::LevelFilter;
use zbus::zvariant::Value;

use zbus::conn;

use crate::config::SystemConfig;
use crate::events::AuthenticationEvent;
use crate::ui::App;

mod authority;
mod config;
mod constants;
mod dbus;
mod events;
mod ui;

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

    let config_path = std::env::var("XDG_CONFIG_HOME")
        .or(std::env::var("HOME").map(|e| e + "/.config"))
        .context("Could not resolve configuration path")?;
    let css_path = format!("{config_path}/soteria/style.css");
    let path = Path::new(&css_path);
    if path.is_file() {
        tracing::info!("loading css stylesheet from {}", css_path);

        let provider = gtk4::CssProvider::new();
        provider.load_from_path(path);
        let display =
            gtk4::gdk::Display::default().ok_or_eyre("Could not get default gtk display.")?;
        gtk4::style_context_add_provider_for_display(&display, &provider, 1000);
    }
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

    let (tx, _rx) = channel::<AuthenticationEvent>(100);

    // Docs say that there are a couple of options for registering ourselves subject
    // wise. Users are having problems with XDG_SESSION_ID not being
    // set on certain desktop environments, so unix-process seems to be preferred
    // (referencing other implementations)
    let locale = "en_US.UTF-8"; // TODO: Needed?
    let subject_kind = "unix-session".to_string();

    let subject_details = HashMap::from([(
        "session-id".to_string(),
        Value::new(
            std::env::var("XDG_SESSION_ID")
                .context("Could not get XDG session id, make sure that it is set and try again.")?,
        ),
    )]);
    let subject = Subject::new(subject_kind, subject_details);

    let agent = AuthenticationAgent::new(tx.clone(), config.clone());
    let connection = conn::Builder::system()?
        .serve_at(constants::SELF_OBJECT_PATH, agent)?
        .build()
        .await?;

    let proxy = AuthorityProxy::new(&connection).await?;
    proxy
        .register_authentication_agent(&subject, locale, constants::SELF_OBJECT_PATH)
        .await?;

    tracing::info!("Registered as authentication provider.");

    let app = RelmApp::new("gay.vaskel.soteria");
    app.run_async::<App>(tx.clone());

    Ok(())
}
