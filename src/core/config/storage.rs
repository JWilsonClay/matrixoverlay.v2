// src/core/config/storage.rs
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Read;
use crate::core::config::Config;

/// [HARDENING] Gets the config path and ensures it is safe from traversal attacks.
pub fn get_config_path() -> Result<(PathBuf, PathBuf)> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    let home_path = Path::new(&home);
    
    // Canonicalize home to resolve any symlink tricks at the root level
    let home_canonical = home_path.canonicalize().context("Failed to canonicalize HOME path")?;
    
    let config_dir = home_canonical.join(".config/matrix-overlay");
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
    } else {
        // [HARDENING] If directory exists, verify it is not a symlink to a sensitive area
        let metadata = fs::symlink_metadata(config_dir).context("Failed to read metadata of config dir")?;
        if metadata.file_type().is_symlink() {
            anyhow::bail!("Security Alert: Config directory is a symlink. This is prohibited.");
        }
    }
    Ok(())
}

pub fn load_raw_config(config_path: &Path) -> Result<Vec<u8>> {
    // [HARDENING] Verify path safety before reading
    if !crate::core::path_utils::is_safe_path(config_path) {
        anyhow::bail!("Security Alert: Attempted to load config from an unsafe path: {:?}", config_path);
    }

    let mut file = fs::File::open(config_path).context("Failed to open config file")?;
    let mut content = Vec::new();
    file.by_ref().take(1024 * 1024).read_to_end(&mut content)?;
    Ok(content)
}

pub fn save_atomic(config: &Config) -> Result<()> {
    let (config_dir, config_path) = get_config_path()?;
    
    // [HARDENING] Ensure we are writing to a directory we own and is safe
    ensure_config_dir(&config_dir)?;
    
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
