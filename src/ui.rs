use gtk::prelude::*;
use gtk4::{
    AlertDialog,
    glib::{clone, spawn_future_local},
    prelude::OrientableExt,
};
use relm4::prelude::*;
use tokio::sync::broadcast;

use crate::events::AuthenticationEvent;

#[derive(Debug)]
pub enum AppMsg {
    Confirm { user: String, password: String },
    Cancel,
    AuthEvent(AuthenticationEvent),
}

pub struct App {
    message: String,
    identities: Vec<String>,
    cookie: Option<String>,
    tx: broadcast::Sender<AuthenticationEvent>, // chosen_identity: Option<String>,
                                                // password_buffer: Option<String>,
}

#[relm4::component(async, pub)]
impl AsyncComponent for App {
    type Input = AppMsg;
    type Output = ();
    type Init = broadcast::Sender<AuthenticationEvent>;
    type CommandOutput = ();

    view! {
        gtk::Window {
            set_title: Some("Authorize"),
            set_default_height: 250,
            set_default_width: 450,
            set_resizable: false,
            set_modal: true,
            set_hide_on_close: true,
            #[watch]
            set_visible: model.cookie.is_some(),

            connect_show[password_entry] => move |_| {
                password_entry.grab_focus();
            },
            connect_close_request[cancel_button] => move |_| {
                cancel_button.emit_clicked();
                gtk4::glib::Propagation::Proceed
            },

            gtk::Box {
                set_margin_end: 56,
                set_margin_start: 56,
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    set_markup: r#"<b><span size='x-large'>Authentication Required</span></b>"#,
                    set_margin_horizontal: 16,
                    set_margin_vertical: 16,
                    set_halign: gtk::Align::Center,
                    set_justify: gtk::Justification::Fill,
                    set_use_markup: true,
                },

                gtk::Label {
                    #[watch]
                    set_label: &model.message,
                    set_single_line_mode: true,
                    set_wrap: true,
                    set_margin_bottom: 16
                },

                gtk::Box {
                    set_baseline_position: gtk::BaselinePosition::Center,
                    set_spacing: 18,

                    #[name = "identity_dropdown"]
                    gtk::DropDown {
                        set_margin_bottom: 8,
                        set_hexpand: true,
                        #[watch]
                        set_model: Some( &gtk::StringList::new(&model.identities.iter().map(AsRef::as_ref).collect::<Vec<_>>()) )
                    }
                },

                #[name = "password_entry"]
                gtk::PasswordEntry {
                    set_hexpand: true,
                    set_placeholder_text: Some( "Password" ),
                    set_show_peek_icon: true,

                    connect_activate[confirm_button] => move |_| {
                        confirm_button.emit_clicked();
                    }
                },

                gtk::FlowBox {
                    set_hexpand: true,
                    set_homogeneous: true,
                    set_max_children_per_line: 2,
                    set_valign: gtk::Align::End,
                    set_vexpand: true,

                    #[name = "cancel_button"]
                    append = &gtk::Button::with_label("Cancel"){
                        connect_clicked[sender, password_entry] => move |_| {

                            sender.input(AppMsg::Cancel);
                            password_entry.set_text("");
                        }
                    },

                    #[name = "confirm_button"]
                    append = &gtk::Button::with_label("Confirm") {
                        connect_clicked[sender, identity_dropdown, password_entry] => move |_| {
                            let user: gtk::StringObject = identity_dropdown.selected_item().unwrap().dynamic_cast().unwrap();

                            sender.input(AppMsg::Confirm { user: user.string().to_string(), password: password_entry.text().to_string()});
                            password_entry.set_text("");

                        }
                    }
                }
            }

        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = App {
            message: String::from(""),
            identities: Vec::new(),
            tx: init,
            cookie: None,
        };

        spawn_future_local(clone!(
            #[strong]
            sender,
            #[strong(rename_to = tx)]
            model.tx,
            async move {
                let mut rx = tx.subscribe();
                loop {
                    let event = rx.recv().await.expect("Somehow the channel closed");
                    tracing::debug!("recieved event {:#?}", event);

                    sender.input(AppMsg::AuthEvent(event));
                }
            }
        ));

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppMsg::Confirm { user, password } => {
                if let Some(cookie) = self.cookie.clone() {
                    self.tx
                        .send(AuthenticationEvent::UserProvidedPassword {
                            cookie,
                            username: user,
                            password,
                        })
                        .unwrap();
                    self.cookie = None;
                    self.message = String::new();
                    self.identities = Vec::new();
                }
            }
            AppMsg::Cancel => {
                if let Some(cookie) = self.cookie.clone() {
                    self.tx
                        .send(AuthenticationEvent::UserCanceled { cookie })
                        .unwrap();
                    self.cookie = None;
                    self.message = String::new();
                    self.identities = Vec::new();
                }
            }
            AppMsg::AuthEvent(ev) => match ev {
                AuthenticationEvent::Started {
                    cookie,
                    message,
                    names,
                } => {
                    if self.cookie.is_none() {
                        self.cookie = Some(cookie);
                        self.message = message;
                        self.identities = names;
                    }
                }
                AuthenticationEvent::Canceled { cookie }
                | AuthenticationEvent::UserCanceled { cookie } => {
                    if let Some(c) = self.cookie.clone() {
                        if c == cookie {
                            self.cookie = None;
                            self.message = String::new();
                            self.identities = Vec::new();
                        }
                    }
                }
                AuthenticationEvent::AuthorizationFailed { .. } => {
                    let alert = build_fail_alert();
                    alert.show(Some(_root));
                    self.cookie = None;
                    self.message = String::new();
                    self.identities = Vec::new();
                }
                _ => (),
            },
        }
    }
}

pub fn build_fail_alert() -> AlertDialog {
    AlertDialog::builder()
        .message("Authentication failed for some reason. Check your login details and try again.")
        .buttons(vec!["Ok"])
        .build()
}
