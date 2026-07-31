use std::fmt::Debug;

#[derive(Clone, zeroize::ZeroizeOnDrop)]
pub enum AuthenticationUserEvent {
    /// The user canceled the authentication.
    Canceled { cookie: String },
    /// Start a PAM conversation. `password` is optional so passwordless PAM
    /// modules can authenticate before a password is requested.
    BeginAuthentication {
        cookie: String,
        username: String,
        password: Option<String>,
    },
    /// Answer a password prompt from an active PAM conversation.
    ProvidedPassword { cookie: String, password: String },
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
    /// PAM requested a password after passwordless methods did not complete.
    PasswordRequested { cookie: String, prompt: String },
    /// Informational text from a PAM module, such as a fingerprint prompt.
    StatusMessage { cookie: String, message: String },
}

// Recursive expansion of Debug macro
// ===================================

impl Debug for AuthenticationUserEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Canceled { cookie } => {
                f.debug_struct("Canceled").field("cookie", &cookie).finish()
            }
            Self::BeginAuthentication {
                cookie, username, ..
            } => f
                .debug_struct("BeginAuthentication")
                .field("cookie", &cookie)
                .field("username", &username)
                .finish(),
            Self::ProvidedPassword { cookie, .. } => f
                .debug_struct("ProvidedPassword")
                .field("cookie", &cookie)
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
            Self::PasswordRequested { cookie, prompt } => f
                .debug_struct("PasswordRequested")
                .field("cookie", cookie)
                .field("prompt", prompt)
                .finish(),
            Self::StatusMessage { cookie, message } => f
                .debug_struct("StatusMessage")
                .field("cookie", cookie)
                .field("message", message)
                .finish(),
        }
    }
}
