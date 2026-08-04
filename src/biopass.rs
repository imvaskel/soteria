//! Optional Biopass biometric integration.
//!
//! Biopass is an optional third-party biometric provider. Its PAM module
//! participates in the regular Polkit PAM conversation, so Soteria only has
//! to start the conversation immediately for Biopass to be invoked. All
//! Biopass-specific behavior is gated behind [`installed`]: when Biopass is
//! not installed none of this code path runs and Soteria falls back to a
//! password-only prompt.

use std::{env, path::Path};

/// Locations where the Biopass PAM module is commonly installed.
const BIOPASS_PAM_PATHS: &[&str] = &[
    "/usr/lib/security/libbiopass_pam.so",
    "/usr/lib/x86_64-linux-gnu/security/libbiopass_pam.so",
    "/lib/security/libbiopass_pam.so",
];

/// Returns `true` when Biopass is installed and can participate in PAM
/// authentication.
pub fn installed() -> bool {
    BIOPASS_PAM_PATHS
        .iter()
        .any(|path| Path::new(path).is_file())
        || helper_on_path()
}

fn helper_on_path() -> bool {
    let path = match env::var_os("PATH") {
        Some(path) => path,
        None => return false,
    };
    env::split_paths(&path).any(|dir| dir.join("biopass-helper").is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_never_panics() {
        // `installed()` reads the host filesystem, so only assert that it
        // returns a boolean without panicking.
        let _ = installed();
    }
}
