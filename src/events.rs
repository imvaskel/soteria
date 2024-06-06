use std::fmt::Debug;

#[derive(Clone)]
pub enum AuthenticationEvent {
    /// A user has requested to authenticate.
    Started {
        cookie: String,
        message: String,
        names: Vec<String>,
    },
    /// Polkit sent a request for the authentication to be canceled.
    Canceled { cookie: String },
    /// The user canceled the authentication.
    UserCanceled { cookie: String },
    /// The user provided their password
    UserProvidedPassword {
        cookie: String,
        username: String,
        password: String,
    },
    /// Authorization failed for some reason.
    AuthorizationFailed { cookie: String },
    // There is already an authentication event being handled.
    //AlreadyRunning { cookie: String },
}

// Recursive expansion of Debug macro
// ===================================

impl Debug for AuthenticationEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AuthenticationEvent::Started {
                cookie,
                message,
                names,
            } => f
                .debug_struct("Started")
                .field("cookie", &cookie)
                .field("message", &message)
                .field("names", &names)
                .finish(),
            AuthenticationEvent::Canceled { cookie } => {
                f.debug_struct("Canceled").field("cookie", &cookie).finish()
            }
            AuthenticationEvent::UserCanceled { cookie } => f
                .debug_struct("UserCanceled")
                .field("cookie", &cookie)
                .finish(),
            AuthenticationEvent::UserProvidedPassword {
                cookie, username, ..
            } => f
                .debug_struct("UserProvidedPassword")
                .field("cookie", &cookie)
                .field("username", &username)
                .finish(),
            AuthenticationEvent::AuthorizationFailed { cookie } => f
                .debug_struct("AuthorizationFailed")
                .field("cookie", &cookie)
                .finish(),
        }
    }
}
