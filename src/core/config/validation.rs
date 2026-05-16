// src/core/config/validation.rs
use anyhow::{bail, Result};
use crate::core::config::Config;
use std::path::Path;

/// [HARDENED] Comprehensive configuration validation with strict bounds.
pub fn validate_config(config: &Config) -> Result<()> {
    // 1. General Bounding
    if config.general.font_size < 8 || config.general.font_size > 72 {
        bail!("font_size must be between 8 and 72");
    }
    if config.general.metric_font_size < 8 || config.general.metric_font_size > 48 {
        bail!("metric_font_size must be between 8 and 48");
    }
    if !is_valid_hex(&config.general.color) {
        bail!("color must be a valid hex string (e.g., #RRGGBB)");
    }
    if config.general.update_ms < 500 || config.general.update_ms > 10000 {
        bail!("update_ms must be between 500 and 10000");
    }
    if config.general.metric_columns < 1 || config.general.metric_columns > 3 {
        bail!("metric_columns must be between 1 and 3");
    }

    // 2. Path Safety (Upgraded from Warn to Bail)
    for (i, screen) in config.screens.iter().enumerate() {
        if screen.x_offset < 0 || screen.y_offset < 0 {
            bail!("Screen {} offsets must be non-negative", i);
        }
        if screen.metrics.len() > 20 {
            bail!("Screen {} has too many metrics (max 20)", i);
        }
    }

    for file in &config.custom_files {
        if !crate::core::path_utils::is_safe_path(Path::new(&file.path)) {
            log::error!("SECURITY ALERT: Unsafe path detected in custom_files: {}", file.path);
            bail!("Unsafe path detected in custom_files");
        }
        if file.name.len() > 32 { bail!("Custom file name too long (max 32)"); }
    }

    for repo in &config.productivity.repos {
        if !crate::core::path_utils::is_safe_path(Path::new(repo)) {
            log::error!("SECURITY ALERT: Unsafe Git repo path: {}", repo);
            bail!("Unsafe path detected in productivity repos");
        }
    }

    // 4. Content Uniqueness Check (75% Threshold)
    validate_uniqueness(config)?;

    // 5. Cosmetic Bounding
    if config.cosmetics.rain_speed < 0.0 || config.cosmetics.rain_speed > 10.0 {
        bail!("rain_speed must be between 0.0 and 10.0");
    }
    if config.cosmetics.realism > 100 {
        bail!("realism must be <= 100");
    }
    if config.cosmetics.metrics_brightness < 0.0 || config.cosmetics.metrics_brightness > 1.0 {
        bail!("metrics_brightness must be between 0.0 and 1.0");
    }
    if config.cosmetics.matrix_brightness < 0.0 || config.cosmetics.matrix_brightness > 1.0 {
        bail!("matrix_brightness must be between 0.0 and 1.0");
    }
    if config.cosmetics.background_opacity < 0.0 || config.cosmetics.background_opacity > 1.0 {
        bail!("background_opacity must be between 0.0 and 1.0");
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

fn validate_uniqueness(config: &Config) -> Result<()> {
    use std::collections::HashSet;
    let mut metric_sets: Vec<HashSet<String>> = Vec::new();
    for screen in &config.screens {
        let mut set = HashSet::new();
        for m in &screen.metrics { set.insert(m.clone()); }
        metric_sets.push(set);
    }

    for i in 0..metric_sets.len() {
        for j in (i + 1)..metric_sets.len() {
            let set_a = &metric_sets[i];
            let set_b = &metric_sets[j];
            let intersection = set_a.intersection(set_b).count();
            let union = set_a.union(set_b).count();
            if union > 0 {
                let similarity = intersection as f64 / union as f64;
                if (1.0 - similarity) < 0.75 {
                    log::warn!("Monitors {} and {} have low uniqueness ({:.1}%).", i, j, (1.0 - similarity) * 100.0);
                }
            }
        }
    }
    Ok(())
}
