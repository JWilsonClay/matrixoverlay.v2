// src/core/config/storage.rs
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;
use std::io::Read;
use crate::core::config::Config;

pub fn get_config_path() -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    let config_dir = Path::new(&home).join(".config/matrix-overlay");
    let config_path = config_dir.join("config.json");
    Ok((config_dir, config_path))
}

pub fn ensure_config_dir(config_dir: &Path) -> Result<()> {
    if !config_dir.exists() {
        fs::create_dir_all(config_dir).context("Failed to create config directory")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(config_dir, fs::Permissions::from_mode(0o700));
        }
    }
    Ok(())
}

pub fn load_raw_config(config_path: &Path) -> Result<Vec<u8>> {
    let mut file = fs::File::open(config_path).context("Failed to open config file")?;
    let mut content = Vec::new();
    file.by_ref().take(1024 * 1024).read_to_end(&mut content)?;
    Ok(content)
}

pub fn save_atomic(config: &Config) -> Result<()> {
    let (config_dir, config_path) = get_config_path()?;
    let temp_path = config_dir.join("config.json.tmp");

    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    
    fs::write(&temp_path, json).context("Failed to write temporary config file")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o600));
    }

    fs::rename(temp_path, config_path).context("Failed to atomize config save via rename")?;
    Ok(())
}
