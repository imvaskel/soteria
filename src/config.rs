use std::path::Path;

use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    helper_path: String,
}

impl SystemConfig {
    pub fn from_file() -> Result<Self, figment::Error> {
        let mut fig = Figment::new();
        // Prioritize configuration in local, as semantically that is the users config
        if Path::new("/usr/local/etc/soteria/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/usr/local/etc/soteria/config.toml"));
            tracing::info!(
                "found configuration at /usr/local/etc/soteria/config.toml, using that."
            );
        // Try the configuration location of the distro
        } else if Path::new("/etc/soteria/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/etc/soteria/config.toml"));
            tracing::info!("found configuration at /etc/soteria/config.toml, using that.");
        // Fall back to default
        } else {
            fig = fig.merge(Serialized::defaults(Self::default()));
            tracing::info!("could not find configuration, using default instead.");
        }
        fig.extract()
    }

    pub fn get_helper_path(&self) -> &str {
        &self.helper_path
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            helper_path: "/usr/lib/polkit-1/polkit-agent-helper-1".into(),
        }
    }
}
