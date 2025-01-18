use authority::{AuthorityProxy, Subject};
use dbus::AuthenticationAgent;
use eyre::{ensure, Result};
use gtk::glib::{self, clone, spawn_future_local};
use state::State;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tokio::sync::broadcast::channel;
use tracing::level_filters::LevelFilter;
use zbus::zvariant::Value;

use gtk::{gio::Cancellable, prelude::*, Builder};
use gtk::{Application, ApplicationWindow, Button, DropDown, Label, PasswordEntry, StringList};
use gtk4 as gtk;
use zbus::conn;

use crate::config::SystemConfig;
use crate::events::AuthenticationEvent;

mod authority;
mod config;
mod constants;
mod dbus;
mod events;
mod state;
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

    gtk::init()?;

    let application = Application::builder()
        .application_id("gay.vaskel.Soteria")
        .build();

    let builder = Builder::from_string(constants::UI_XML);

    let window: ApplicationWindow = ui::get_object(&builder, "window")?;
    let password_entry: PasswordEntry = ui::get_object(&builder, "password-entry")?;
    let cancel_button: Button = ui::get_object(&builder, "cancel-button")?;
    let confirm_button: Button = ui::get_object(&builder, "confirm-button")?;
    let info_label: Label = ui::get_object(&builder, "label-message")?;
    let dropdown: DropDown = ui::get_object(&builder, "identity-dropdown")?;

    application.connect_activate(clone!(@weak window => move |app| {
        app.add_window(&window);
    }));

    password_entry.connect_activate(clone!(@weak confirm_button => move |_| {
        confirm_button.emit_clicked();
    }));

    let (tx, mut rx) = channel::<AuthenticationEvent>(100);

    // Docs say that there are a couple of options for registering ourselves subject
    // wise. Users are having problems with XDG_SESSION_ID not being
    // set on certain desktop environments, so unix-process seems to be preferred
    // (referencing other implementations)
    let locale = "en_US.UTF-8"; // TODO: Needed?
    let subject_kind = "unix-process".to_string();
    let subject_details = HashMap::from([
        ("pid".to_string(), Value::new(std::process::id())),
        ("start-time".to_string(), 0),
        ("uid".to_string(), -1),
    ]);
    let subject = Subject::new(subject_kind, subject_details);

    application.register(Cancellable::NONE)?;
    application.activate();

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

    spawn_future_local(clone!(@weak window, @weak builder => async move {
        let mut state = State::new(tx.clone(), cancel_button.clone(), confirm_button.clone(), password_entry.clone(), window.clone(), dropdown.clone());

        loop {
            let failed_alert = ui::build_fail_alert();

            let event = rx.recv().await.expect("Somehow the channel closed.");
            tracing::debug!("recieved event {:#?}", event);

            match event {
                AuthenticationEvent::Started{cookie, message, names} => {
                    let res = state.start_authentication(cookie).unwrap();
                    if !res {
                        continue;
                    }

                    let store: StringList = builder.object("identity-dropdown-values").unwrap();
                    for name in names.iter() {
                        store.append(name.as_str());
                    }
                    info_label.set_label(&message);

                    tracing::debug!("Attempting to prompt user for authentication.");
                    window.present();
                }
                AuthenticationEvent::Canceled{cookie: c} => {
                    state.end_authentication(&c);
                },
                AuthenticationEvent::UserProvidedPassword{ cookie: c, username: _, password: _} => {
                    state.end_authentication(&c);
                }
                AuthenticationEvent::AuthorizationFailed{cookie: c} => {
                    state.end_authentication(&c);
                    failed_alert.show(Some(&window));
                }
                _ => (),
            }
        }
    }));

    application.run();

    Ok(())
}
