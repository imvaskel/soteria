#[derive(Debug, Clone)]
pub enum AuthenticationEvent {
    /// A user has requested to authenticate.
    Started(String, String, Vec<String>),
    /// Polkit sent a request for the authentication to be canceled.
    Cancelled(String),
    /// The user canceled the authentication.
    UserCancelled(String),
    /// The user provided their password
    UserProvidedPassword(String, String, String),
    /// Authorization failed for some reason.
    AuthorizationFailed(String),
}
