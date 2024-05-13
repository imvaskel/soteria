use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SystemConfig {
    helper_path: String,
}

impl SystemConfig {
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
