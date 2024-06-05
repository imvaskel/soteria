use std::path::Path;

use eyre::Result;
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
    pub fn from_file() -> Result<Self> {
        let mut fig = Figment::new();
        // Prioritize configuration in local, as semantically that is the users config
        if Path::new("/usr/local/etc/soteria/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/usr/local/etc/soteria/config.toml"));
            tracing::info!("using configuration file found at /usr/local/etc/soteria/config.toml");
        // Try the configuration location of the distro
        } else if Path::new("/etc/soteria/config.toml").exists() {
            fig = fig.merge(Toml::file_exact("/etc/soteria/config.toml"));
            tracing::info!("using configuration file found at /etc/soteria/config.toml");
        // Fall back to default
        } else {
            fig = fig.merge(Serialized::defaults(Self::default()));
            tracing::info!("no configuration file found, using default configuration instead");
        }
        Ok(fig.extract()?)
    }

    pub fn get_helper_path(&self) -> &str {
        &self.helper_path
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            helper_path: env!("POLKIT_AGENT_HELPER_PATH").into(),
        }
    }
}
