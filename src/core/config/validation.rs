// src/core/config/validation.rs
use anyhow::{bail, Result};
use crate::core::config::Config;
use std::path::Path;

pub fn validate_config(config: &Config) -> Result<()> {
    if config.general.font_size < 12 {
        bail!("font_size must be >= 12");
    }
    if !is_valid_hex(&config.general.color) {
        bail!("color must be a valid hex string (e.g., #RRGGBB)");
    }
    if config.general.update_ms < 500 {
        bail!("update_ms must be >= 500");
    }
    for (i, screen) in config.screens.iter().enumerate() {
        if screen.x_offset < 0 || screen.y_offset < 0 {
            bail!("Screen {} offsets must be non-negative", i);
        }
    }

    for file in &config.custom_files {
        if !crate::core::path_utils::is_safe_path(Path::new(&file.path)) {
            log::warn!("Security Warning: Unsafe path detected in custom_files: {}", file.path);
        }
    }
    for repo in &config.productivity.repos {
        if !crate::core::path_utils::is_safe_path(Path::new(repo)) {
            log::warn!("Security Warning: Unsafe Git repo path: {}", repo);
        }
    }

    Ok(())
}

fn is_valid_hex(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}
