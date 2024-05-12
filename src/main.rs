use authority::{AuthorityProxy, Subject};
use dbus::AuthenticationAgent;
use gtk::glib::{self, clone, spawn_future_local, SignalHandlerId};
use std::collections::HashMap;
use tokio::sync::broadcast::channel;
use zbus::zvariant::Value;

use gtk::{gio::Cancellable, prelude::*, Builder};
use gtk::{
    Application, ApplicationWindow, Button, DropDown, Label, PasswordEntry, StringList,
    StringObject,
};
use gtk4 as gtk;
use zbus::conn;

use crate::events::AuthenticationEvent;

mod authority;
mod constants;
mod dbus;
mod events;
mod ui;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(
                "zbus::object_server::ObjectServer::start_object_server[start_object_server]=debug"
                    .parse()
                    .unwrap(),
            ),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    match real_main().await {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("A fatal error occurred when running the application: {}", e);
            std::process::exit(1);
        }
    }
}
async fn real_main() -> Result<(), Box<dyn std::error::Error>> {
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

    let w2 = window.clone();
    application.connect_activate(move |app| {
        app.add_window(&w2);
    });

    let (tx, mut rx) = channel::<AuthenticationEvent>(100);

    let locale = "en_US.UTF-8";
    let subject_kind = "unix-session".to_string();
    let subject_details = HashMap::from([(
        "session-id".to_string(),
        Value::new(std::env::var("XDG_SESSION_ID")?),
    )]);
    let subject = Subject::new(subject_kind, subject_details);

    application.register(Cancellable::NONE)?;
    application.activate();

    let agent = AuthenticationAgent::new(tx.clone());
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
        // Mapping of cookie::confirm button listener, cancel button listener
        let mut registered_listeners: HashMap<String, (SignalHandlerId, SignalHandlerId)> = HashMap::new();

        loop {
            let dropdown: DropDown = builder.object("identity-dropdown").unwrap();
            let failed_alert = ui::build_fail_alert();

            let event = rx.recv().await.expect("Somehow the channel closed.");
            tracing::debug!("recieved event {:#?}", event);

            match event {
                AuthenticationEvent::Started(cookie, message, names) => {
                    let store: StringList = builder.object("identity-dropdown-values").unwrap();
                    for name in names.iter() {
                        store.append(name.as_str());
                    }
                    info_label.set_label(&message);

                    let cancel_listener = cancel_button.connect_clicked(clone!(@weak window, @weak password_entry, @weak info_label, @strong cookie, @strong tx => move |_| {
                        tx.send(AuthenticationEvent::UserCancelled(cookie.clone())).unwrap();
                        password_entry.set_text("");
                        info_label.set_text("");
                        window.set_visible(false);
                    }));

                    let confirm_listener = confirm_button.connect_clicked(clone!(@weak window, @weak password_entry, @weak info_label, @strong cookie, @strong tx => move |_| {
                        let pw = password_entry.text();
                        let user: StringObject = dropdown.selected_item().unwrap().dynamic_cast().unwrap();
                        tx.send(AuthenticationEvent::UserProvidedPassword(cookie.clone(), user.string().to_string(), pw.to_string())).unwrap();
                        password_entry.set_text("");
                        info_label.set_text("");
                        window.set_visible(false);
                    }));

                    registered_listeners.insert(cookie.clone(), (confirm_listener, cancel_listener));
                    tracing::debug!("Attempting to prompt user for authentication.");
                    window.present();
                }
                AuthenticationEvent::Cancelled(c) => {
                    match registered_listeners.remove(&c) {
                        Some((confirm, cancel)) => {
                            cancel_button.disconnect(cancel);
                            confirm_button.disconnect(confirm);
                            tracing::debug!("removed listeners from buttons.");
                        }
                        None => tracing::debug!("have cookie that was not registered to any listeners.")
                    }

                    window.set_visible(false);
                },
                AuthenticationEvent::UserProvidedPassword(c, _, _) => {
                    match registered_listeners.remove(&c) {
                        Some((confirm, cancel)) => {
                            cancel_button.disconnect(cancel);
                            confirm_button.disconnect(confirm);
                            tracing::debug!("removed listeners from buttons.");
                        }
                        None => tracing::debug!("have cookie that was not registered to any listeners.")
                    }
                }
                AuthenticationEvent::AuthorizationFailed(_) => {
                    failed_alert.show(Some(&window));
                }
                _ => (),
            }
        }
    }));

    application.run();

    Ok(())
}
