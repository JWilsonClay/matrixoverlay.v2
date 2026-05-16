// src/core/config/mod.rs
pub mod defaults;
pub mod storage;
pub mod validation;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use crate::core::config::defaults::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct General {
    pub font_size: u32,
    #[serde(default = "default_metric_font_size")]
    pub metric_font_size: u32,
    pub color: String,
    pub update_ms: u64,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_glow_passes")]
    pub glow_passes: Vec<(f64, f64, f64)>,
    #[serde(default = "default_true")]
    pub show_monitor_label: bool,
    #[serde(default = "default_spacing")]
    pub metric_spacing: i32,
    #[serde(default = "default_columns")]
    pub metric_columns: u32,
    #[serde(default = "default_alignment")]
    pub metric_alignment: String,
    #[serde(default = "default_label_spacing")]
    pub label_value_spacing: i32,
    #[serde(default)]
    pub show_cpu_metric: bool,
    #[serde(default = "default_temp_unit")]
    pub temp_unit: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Screen {
    pub metrics: Vec<String>,
    pub x_offset: i32,
    pub y_offset: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Weather {
    pub lat: f64,
    pub lon: f64,
    pub enabled: bool,
    #[serde(default = "default_false")]
    pub auto_location: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomFile {
    pub name: String,
    pub path: String,
    pub metric_id: String,
    #[serde(default)]
    pub tail: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Productivity {
    #[serde(default)]
    pub repos: Vec<String>,
    #[serde(default = "default_commit_threshold")]
    pub auto_commit_threshold: u64,
    #[serde(default)]
    pub ollama_enabled: bool,
    #[serde(default = "default_batch_cap")]
    pub batch_cap: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cosmetics {
    #[serde(default = "default_rain_mode")]
    pub rain_mode: String,
    #[serde(default = "default_realism")]
    pub realism_scale: u32,
    #[serde(default = "default_true")]
    pub occlusion_enabled: bool,
    #[serde(default = "default_rain_speed")]
    pub rain_speed: f64,
    #[serde(default = "default_brightness")]
    pub metrics_brightness: f64,
    #[serde(default = "default_brightness")]
    pub matrix_brightness: f64,
    #[serde(default)]
    pub border_enabled: bool,
    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default = "default_bg_opacity")]
    pub background_opacity: f64,
    #[serde(default = "default_preset")]
    pub perf_preset: String,
}

impl Default for Cosmetics {
    fn default() -> Self {
        Self {
            rain_mode: default_rain_mode(),
            realism_scale: default_realism(),
            occlusion_enabled: default_true(),
            rain_speed: default_rain_speed(),
            metrics_brightness: default_brightness(),
            matrix_brightness: default_brightness(),
            border_enabled: false,
            border_color: default_border_color(),
            background_opacity: default_bg_opacity(),
            perf_preset: default_preset(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Logging {
    pub enabled: bool,
    pub log_path: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    #[serde(default = "default_max_size")]
    pub max_file_size_mb: u64,
    #[serde(default)]
    pub build_logging_enabled: bool,
}

impl Default for Logging {
    fn default() -> Self {
        Self { 
            enabled: false, 
            log_path: "/tmp/matrix_overlay_logs/".to_string(),
            interval_secs: 30,
            max_files: 5,
            max_file_size_mb: 1,
            build_logging_enabled: true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

    pub fn save(&self) -> Result<()> {
        storage::save_atomic(self)
    }

    pub fn validate(&self) -> Result<()> {
        validation::validate_config(self)
    }
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
        for screen in &config.screens {
            for m in &screen.metrics {
                if !config.weather.enabled && (m == "weather_temp" || m == "weather_condition") { continue; }
                metrics.insert(m.clone());
            }
        }
        Self {
            refresh_rate_ms: config.general.update_ms,
            enable_nvidia: true,
            active_metrics: metrics.into_iter().collect(),
            latitude: config.weather.lat,
            longitude: config.weather.lon,
        }
    }
}
