use std::fmt::Debug;

#[derive(Clone, zeroize::ZeroizeOnDrop)]
pub enum AuthenticationUserEvent {
    /// The user canceled the authentication.
    Canceled { cookie: String },
    /// The user provided their password.
    ProvidedPassword {
        cookie: String,
        username: String,
        password: String,
    },
}

#[derive(Clone, zeroize::ZeroizeOnDrop)]
pub enum AuthenticationAgentEvent {
    /// Agent has begun authentication.
    Started {
        cookie: String,
        message: String,
        names: Vec<String>,
    },
    /// Polkit sent a request for the authentication to be canceled.
    Canceled { cookie: String },
    /// The user has successfully authenticated.
    AuthorizationSucceeded { cookie: String },
    // There is already an authentication event being handled.
    //AlreadyRunning { cookie: String },
    /// The user provided a password, but it was incorrect.
    AuthorizationRetry {
        cookie: String,
        retry_message: Option<String>,
    },
}

// Recursive expansion of Debug macro
// ===================================

impl Debug for AuthenticationUserEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Canceled { cookie } => {
                f.debug_struct("Canceled").field("cookie", &cookie).finish()
            }
            Self::ProvidedPassword {
                cookie, username, ..
            } => f
                .debug_struct("ProvidedPassword")
                .field("cookie", &cookie)
                .field("username", &username)
                .finish(),
        }
    }
}

impl Debug for AuthenticationAgentEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Started {
                cookie,
                message,
                names,
            } => f
                .debug_struct("Started")
                .field("cookie", &cookie)
                .field("message", &message)
                .field("names", &names)
                .finish(),
            Self::Canceled { cookie } => {
                f.debug_struct("Canceled").field("cookie", &cookie).finish()
            }
            Self::AuthorizationSucceeded { cookie } => f
                .debug_struct("AuthorizationSucceeded")
                .field("cookie", &cookie)
                .finish(),
            Self::AuthorizationRetry {
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
