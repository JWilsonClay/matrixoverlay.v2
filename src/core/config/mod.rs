// src/core/config/mod.rs
pub mod defaults;
pub mod storage;
pub mod validation;
pub mod types;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use crate::core::config::defaults::*;
pub use self::types::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub general: General,
    pub screens: Vec<Screen>,
    pub weather: Weather,
    #[serde(default)]
    pub custom_files: Vec<CustomFile>,
    #[serde(default)]
    pub productivity: Productivity,
    #[serde(default)]
    pub cosmetics: Cosmetics,
    #[serde(default)]
    pub logging: Logging,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General {
                font_size: 14,
                metric_font_size: 14,
                color: "#00FF41".to_string(),
                update_ms: 1000,
                theme: "classic".to_string(),
                glow_passes: default_glow_passes(),
                show_monitor_label: true,
                metric_spacing: 24,
                metric_columns: 1,
                metric_alignment: "left".to_string(),
                label_value_spacing: 10,
                show_cpu_metric: false,
                temp_unit: "celsius".to_string(),
                header_font_size: default_header_font_size(),
            },
            screens: vec![Screen {
                metrics: vec!["cpu_usage".to_string(), "ram_usage".to_string(), "disk_usage".to_string(), "network_details".to_string(), "cpu_temp".to_string(), "gpu_temp".to_string()],
                x_offset: 20,
                y_offset: 20,
            }],
            weather: Weather { lat: 0.0, lon: 0.0, enabled: false, auto_location: false },
            custom_files: Vec::new(),
            productivity: Productivity::default(),
            cosmetics: Cosmetics::default(),
            logging: Logging::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let (config_dir, config_path) = storage::get_config_path()?;
        
        if !config_path.exists() {
            storage::ensure_config_dir(&config_dir)?;
            let default_config = Config::default();
            let json = serde_json::to_string_pretty(&default_config).context("Failed to serialize default config")?;
            fs::write(&config_path, json).context("Failed to write default config file")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600));
            }
            return Ok(default_config);
        }

        let content = storage::load_raw_config(&config_path)?;
        let config: Config = serde_json::from_slice(&content).context("Failed to parse config.json")?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> { storage::save_atomic(self) }
    pub fn validate(&self) -> Result<()> { validation::validate_config(self) }
}

// Compatibility struct
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub refresh_rate_ms: u64,
    pub enable_nvidia: bool,
    pub active_metrics: Vec<String>,
    pub latitude: f64,
    pub longitude: f64,
}

impl From<&Config> for MetricsConfig {
    fn from(config: &Config) -> Self {
        let mut metrics = std::collections::HashSet::new();
        let mut nvidia_needed = false;

        for screen in &config.screens {
            for m in &screen.metrics {
                if !config.weather.enabled && (m == "weather_temp" || m == "weather_condition") { continue; }
                if m == "gpu_temp" || m == "gpu_usage" { nvidia_needed = true; }
                metrics.insert(m.clone());
            }
        }
        Self {
            refresh_rate_ms: config.general.update_ms,
            enable_nvidia: nvidia_needed,
            active_metrics: metrics.into_iter().collect(),
            latitude: config.weather.lat,
            longitude: config.weather.lon,
        }
    }
}
