use gettextrs::gettext;
use gtk::prelude::*;
use gtk4::{
    glib::{clone, spawn_future_local},
    prelude::OrientableExt,
};
use relm4::prelude::*;
use tokio::sync::mpsc;

use crate::events::{AuthenticationAgentEvent, AuthenticationUserEvent};

#[derive(Debug, zeroize::ZeroizeOnDrop)]
pub enum AppMsg {
    Confirm { user: String, password: String },
    Cancel,
    AuthEvent(AuthenticationAgentEvent),
}

pub struct App {
    message: String,
    identities: Vec<String>,
    cookie: Option<String>,
    retry_message: Option<String>,
    authenticating: bool,
    sender: mpsc::Sender<AuthenticationUserEvent>, // chosen_identity: Option<String>,
}

#[relm4::component(async, pub)]
impl AsyncComponent for App {
    type Input = AppMsg;
    type Output = ();
    type Init = (
        mpsc::Sender<AuthenticationUserEvent>,
        mpsc::Receiver<AuthenticationAgentEvent>,
    );
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
                    set_markup: &format!(r#"<b><span size='x-large'>{}</span></b>"#, gettext("Authentication Required")),
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
                    #[watch]
                    set_margin_bottom: if model.retry_message.is_some() { 8 } else { 16 },
                },

                gtk::Label {
                    #[watch]
                    set_label: &if let Some(retry_message) = model.retry_message.clone() {
                        retry_message.to_string()
                    } else {
                        "".to_string()
                    },
                    #[watch]
                    set_visible: model.retry_message.is_some(),
                    #[watch]
                    set_margin_bottom: 16,
                    set_halign: gtk::Align::Center,
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
                    set_placeholder_text: Some( &gettext("Password") ),
                    set_show_peek_icon: true,
                    #[watch]
                    set_editable: !model.authenticating,

                    connect_activate[confirm_button] => move |_| {
                        confirm_button.emit_clicked();
                    }
                },

                gtk::FlowBox {
                    set_hexpand: true,
                    set_homogeneous: true,
                    set_margin_bottom: 16,
                    set_margin_top: 8,
                    set_max_children_per_line: 2,
                    set_valign: gtk::Align::End,
                    set_vexpand: true,

                    #[name = "cancel_button"]
                    append = &gtk::Button::with_label(&gettext("Cancel")){
                        connect_clicked[sender, password_entry] => move |_| {

                            sender.input(AppMsg::Cancel);
                            password_entry.set_text("");
                        }
                    },

                    #[name = "confirm_button"]
                    append = &gtk::Button::with_label(&gettext("Confirm")) {
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
            sender: init.0,
            cookie: None,
            authenticating: false,
            retry_message: None,
        };

        spawn_future_local(clone!(
            #[strong]
            sender,
            async move {
                let mut receiver = init.1;
                loop {
                    let event = receiver.recv().await.expect("Somehow the channel closed");
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
        match &message {
            AppMsg::Confirm { user, password } => {
                if let Some(cookie) = self.cookie.clone() {
                    self.sender
                        .send(AuthenticationUserEvent::ProvidedPassword {
                            cookie,
                            username: user.clone(),
                            password: password.clone(),
                        })
                        .await
                        .unwrap();
                    self.retry_message = Some(gettext("Authenticating..."));
                    self.authenticating = true;
                }
            }
            AppMsg::Cancel => {
                if let Some(cookie) = self.cookie.clone() {
                    self.sender
                        .send(AuthenticationUserEvent::Canceled { cookie })
                        .await
                        .unwrap();
                    self.cookie = None;
                    self.message = String::new();
                    self.retry_message = Some(String::new());
                    self.authenticating = false;
                    self.identities = Vec::new();
                }
            }
            AppMsg::AuthEvent(ev) => match ev {
                AuthenticationAgentEvent::Started {
                    cookie,
                    message,
                    names,
                } => {
                    if self.cookie.is_none() {
                        self.cookie = Some(cookie.clone());
                        self.message = message.clone();
                        self.identities = names.clone();
                        self.authenticating = false;
                        self.retry_message = None;
                    }
                }
                AuthenticationAgentEvent::Canceled { cookie } => {
                    if let Some(c) = &self.cookie {
                        if c == cookie {
                            self.cookie = None;
                            self.message = String::new();
                            self.identities = Vec::new();
                            self.retry_message = None;
                            self.authenticating = false;
                        }
                    }
                }
                AuthenticationAgentEvent::AuthorizationSucceeded { cookie } => {
                    if let Some(c) = &self.cookie {
                        if c == cookie {
                            tracing::debug!("Authentication succeeded, closing window.");
                            self.cookie = None;
                            self.message.clear();
                            self.identities.clear();
                            self.retry_message = None;
                            self.authenticating = false;
                        }
                    }
                }
                AuthenticationAgentEvent::AuthorizationRetry {
                    cookie,
                    retry_message,
                } => {
                    if let Some(c) = &self.cookie {
                        if c == cookie {
                            self.retry_message = retry_message.clone();
                            self.authenticating = false;
                        }
                    }
                }
            },
        }
    }
}
