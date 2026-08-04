use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use eyre::{Result, bail, eyre};
use tokio::sync::mpsc;
use zbus::{Connection, Proxy, interface};

use crate::constants;

#[derive(Debug)]
pub enum ThemeRequest {
    Reload,
    Load(PathBuf),
}

#[derive(Clone)]
pub struct ThemeControl {
    current_path: Arc<Mutex<PathBuf>>,
    sender: mpsc::UnboundedSender<ThemeRequest>,
}

impl ThemeControl {
    pub fn new(
        current_path: Arc<Mutex<PathBuf>>,
        sender: mpsc::UnboundedSender<ThemeRequest>,
    ) -> Self {
        Self {
            current_path,
            sender,
        }
    }
}

#[interface(name = "gay.vaskel.Soteria.Theme1")]
impl ThemeControl {
    fn path(&self) -> zbus::fdo::Result<String> {
        self.current_path
            .lock()
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|_| zbus::fdo::Error::Failed("theme path lock is poisoned".into()))
    }

    fn reload(&self) -> zbus::fdo::Result<()> {
        self.sender
            .send(ThemeRequest::Reload)
            .map_err(|_| zbus::fdo::Error::Failed("theme controller is unavailable".into()))
    }

    fn load(&self, path: String) -> zbus::fdo::Result<()> {
        let path = PathBuf::from(path);
        validate_style_path(&path).map_err(|error| {
            zbus::fdo::Error::InvalidArgs(format!("invalid stylesheet: {error}"))
        })?;
        self.sender
            .send(ThemeRequest::Load(path))
            .map_err(|_| zbus::fdo::Error::Failed("theme controller is unavailable".into()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ThemeCommand {
    Path,
    Reload,
    Load(PathBuf),
}

pub fn command_from_args(args: &[String]) -> Result<Option<ThemeCommand>> {
    if args.first().map(String::as_str) != Some("theme") {
        return Ok(None);
    }

    match args.get(1).map(String::as_str) {
        Some("path") if args.len() == 2 => Ok(Some(ThemeCommand::Path)),
        Some("reload") if args.len() == 2 => Ok(Some(ThemeCommand::Reload)),
        Some("load") if args.len() == 3 => Ok(Some(ThemeCommand::Load(PathBuf::from(&args[2])))),
        _ => bail!("usage: soteria theme <path|reload|load /absolute/path/theme.css>"),
    }
}

pub fn default_style_path() -> Result<PathBuf> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))
        .ok_or_else(|| eyre!("could not resolve configuration path"))?;
    Ok(config_home.join("soteria/style.css"))
}

pub fn validate_style_path(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("path must be absolute");
    }
    if !path.is_file() {
        bail!("{} is not a regular file", path.display());
    }
    Ok(())
}

pub async fn run_command(command: ThemeCommand) -> Result<()> {
    let connection = Connection::session().await?;
    let proxy = Proxy::new(
        &connection,
        constants::THEME_BUS_NAME,
        constants::THEME_OBJECT_PATH,
        constants::THEME_INTERFACE,
    )
    .await?;

    match command {
        ThemeCommand::Path => {
            let path: String = proxy.call("Path", &()).await?;
            println!("{path}");
        }
        ThemeCommand::Reload => {
            proxy.call::<_, _, ()>("Reload", &()).await?;
        }
        ThemeCommand::Load(path) => {
            validate_style_path(&path)?;
            proxy
                .call::<_, _, ()>("Load", &(path.to_string_lossy().as_ref()))
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn ignores_agent_invocation() {
        assert_eq!(command_from_args(&[]).unwrap(), None);
    }

    #[test]
    fn parses_theme_commands() {
        assert_eq!(
            command_from_args(&args(&["theme", "path"])).unwrap(),
            Some(ThemeCommand::Path)
        );
        assert_eq!(
            command_from_args(&args(&["theme", "reload"])).unwrap(),
            Some(ThemeCommand::Reload)
        );
        assert_eq!(
            command_from_args(&args(&["theme", "load", "/tmp/theme.css"])).unwrap(),
            Some(ThemeCommand::Load(PathBuf::from("/tmp/theme.css")))
        );
    }

    #[test]
    fn rejects_incomplete_commands() {
        assert!(command_from_args(&args(&["theme", "load"])).is_err());
        assert!(command_from_args(&args(&["theme", "unknown"])).is_err());
    }
}
