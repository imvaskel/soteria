use std::path::Path;

use eyre::Result;
use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    helper_path: String,
    socket_path: String,
}

impl SystemConfig {
    pub fn from_file() -> Result<Self> {
        let mut fig = Figment::new();
        // Prioritize XDG_CONFIG_HOME (user config), defaulting to $HOME/.config
        let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME")
            .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config").into()));
        let xdg_path = xdg_config_home.map(|c| Path::new(&c).join("soteria/config.toml"));

        if xdg_path.as_ref().is_some_and(|p| p.exists()) {
            let path = xdg_path.unwrap();
            fig = fig.merge(Toml::file_exact(path.clone()));
            tracing::info!("using configuration file found at {}", path.display());
        }
        // Prioritize configuration in local, as semantically that is the users config
        else if Path::new("/usr/local/etc/soteria/config.toml").exists() {
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

    pub fn get_socket_path(&self) -> &str {
        &self.socket_path
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            helper_path: env!("POLKIT_AGENT_HELPER_PATH").into(),
            socket_path: env!("POLKIT_AGENT_SOCKET_PATH").into(),
        }
    }
}
