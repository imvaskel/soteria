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
    /// The user has successfully authenticated.
    AuthorizationSucceeded { cookie: String },

    // There is already an authentication event being handled.
    //AlreadyRunning { cookie: String },
    /// The user provided a password, but it was incorrect.
    AuthorizationRetry {
        cookie: String,
        retry_message: String,
    },
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
            AuthenticationEvent::AuthorizationSucceeded { cookie } => f
                .debug_struct("AuthorizationSucceeded")
                .field("cookie", &cookie)
                .finish(),
            AuthenticationEvent::AuthorizationRetry {
                cookie,
                retry_message,
            } => f
                .debug_struct("AuthorizationRetry")
                .field("cookie", &cookie)
                .field("retry_message", &retry_message)
                .finish(),
        }
    }
}
