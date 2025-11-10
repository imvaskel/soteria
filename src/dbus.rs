use std::{collections::HashMap, process::Stdio};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process,
    sync::broadcast,
};
use zbus::{interface, zvariant::Value};

use crate::{
    authority::{Identity, PolkitError, Result},
    config::SystemConfig,
    events::AuthenticationEvent,
};

#[derive(Debug)]
pub struct AuthenticationAgent {
    config: SystemConfig,
    sender: broadcast::Sender<AuthenticationEvent>,
}

impl AuthenticationAgent {
    pub fn new(sender: broadcast::Sender<AuthenticationEvent>, config: SystemConfig) -> Self {
        Self { sender, config }
    }
}

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl AuthenticationAgent {
    async fn cancel_authentication(&self, cookie: &str) {
        tracing::debug!("Recieved request to cancel authentication for {}", cookie);
        let tx = self.sender.clone();
        tx.send(AuthenticationEvent::Canceled {
            cookie: cookie.to_owned(),
        })
        .unwrap();
    }

    async fn begin_authentication(
        &self,
        action_id: &str,
        message: &str,
        icon_name: &str,
        details: HashMap<String, String>,
        cookie: &str,
        identities: Vec<Identity<'_>>,
    ) -> Result<()> {
        tracing::info!("recieved request to authenticate");
        tracing::debug!(action_id = action_id, message = message, icon_name = icon_name, details = ?details, cookie = cookie, identities = ?identities);

        let mut names: Vec<String> = Vec::new();
        for identity in identities.iter() {
            let details = identity.get_details();
            if identity.get_kind() == "unix-user" {
                let Value::U32(uid) = details["uid"] else {
                    continue;
                };
                if let Ok(Some(u)) = etc_passwd::Passwd::from_uid(uid) {
                    if let Ok(n) = u.name.into_string() {
                        names.push(n);
                    }
                }
            }
        }

        self.sender
            .send(AuthenticationEvent::Started {
                cookie: cookie.to_string(),
                message: message.to_string(),
                retry_message: None,
                names,
            })
            .map_err(|_| PolkitError::Failed("Failed to send data.".to_string()))?;

        let mut rx = self.sender.subscribe();

        loop {
            match rx
                .recv()
                .await
                .map_err(|_| PolkitError::Failed("Failed to recieve data.".to_string()))?
            {
                AuthenticationEvent::UserCanceled { cookie: c } => {
                    if c == cookie {
                        return Err(PolkitError::Cancelled(
                            "User cancelled the authentication.".to_string(),
                        ));
                    }
                }
                AuthenticationEvent::UserProvidedPassword {
                    cookie: c,
                    username: user,
                    password: pw,
                } => {
                    if c == cookie {
                        let mut child = process::Command::new(self.config.get_helper_path())
                            .arg(&user)
                            .stdin(Stdio::piped())
                            .stdout(Stdio::piped())
                            .spawn()
                            .map_err(|_| {
                                PolkitError::Failed(
                                    "Failed to the spawn polkit authentication helper.".to_string(),
                                )
                            })?;

                        let mut stdin = child
                            .stdin
                            .take()
                            .ok_or(PolkitError::Failed("Child did not have stdin.".to_string()))?;
                        let stdout = child.stdout.take().ok_or(PolkitError::Failed(
                            "Child did not have stdout.".to_string(),
                        ))?;

                        stdin.write_all(cookie.as_bytes()).await?;
                        stdin.write_all(b"\n").await?;

                        let mut last_info: Option<String> = None;

                        let reader = BufReader::new(stdout);
                        let mut lines = reader.lines();
                        while let Some(line) = lines.next_line().await? {
                            tracing::debug!("helper stdout: {}", line);
                            if let Some(sliced) = line.strip_prefix("PAM_PROMPT_ECHO_OFF") {
                                tracing::debug!("recieved request from helper: '{}'", sliced);
                                if sliced.trim() == "Password:" {
                                    tracing::debug!(pw = pw);
                                    stdin.write_all(pw.as_bytes()).await?;
                                    stdin.write_all(b"\n").await?;
                                }
                            } else if let Some(info) = line.strip_prefix("PAM_TEXT_INFO") {
                                let msg = info.trim().to_string();
                                tracing::debug!("helper replied with info: {}", msg);

                                if msg.contains("minute") && msg.contains("unlock") {
                                    last_info = Some(msg.clone());
                                    self.sender
                                        .send(AuthenticationEvent::AuthorizationRetry {
                                            cookie: cookie.to_string(),
                                            retry_message: Some(msg),
                                        })
                                        .unwrap();
                                }
                            } else if line.starts_with("FAILURE") {
                                tracing::debug!("helper replied with failure.");

                                let retry_msg = last_info.clone().unwrap_or_else(|| {
                                    "Authentication failed. Please try again.".to_string()
                                });
                                self.sender
                                    .send(AuthenticationEvent::AuthorizationRetry {
                                        cookie: cookie.to_string(),
                                        retry_message: Some(retry_msg),
                                    })
                                    .unwrap();
                                continue;
                            } else if line.starts_with("SUCCESS") {
                                tracing::debug!("helper replied with success.");

                                self.sender
                                    .send(AuthenticationEvent::AuthorizationSucceeded {
                                        cookie: cookie.to_string(),
                                    })
                                    .unwrap();
                                return Ok(());
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
