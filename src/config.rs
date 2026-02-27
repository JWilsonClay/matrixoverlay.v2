//! Configuration management.
//! Handles loading and parsing of config.json.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct General {
    pub font_size: u32,
    pub color: String,
    pub update_ms: u64,
    #[serde(default = "default_glow_passes")]
    pub glow_passes: Vec<(f64, f64, f64)>,
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
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomFile {
    pub name: String,      // Display label (e.g. "Server Log")
    pub path: String,      // Path to file (e.g. "/mnt/shared/status.txt")
    pub metric_id: String, // ID to use in screen config (e.g. "server_status")
    #[serde(default)]
    pub tail: bool,        // If true, only display the last line of the file
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

fn default_commit_threshold() -> u64 { 1000 }
fn default_batch_cap() -> u32 { 5 }

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Cosmetics {
    #[serde(default = "default_rain_mode")]
    pub rain_mode: String, // "fall", "pulse", "off"
    #[serde(default = "default_realism")]
    pub realism_scale: u32, // 0-10
    #[serde(default = "default_true")]
    pub occlusion_enabled: bool,
}

fn default_rain_mode() -> String { "off".to_string() }
fn default_realism() -> u32 { 5 }
fn default_true() -> bool { true }

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
}

fn default_glow_passes() -> Vec<(f64, f64, f64)> {
    vec![
        (-2.0, -2.0, 0.2),
        (-1.0, -1.0, 0.3),
        (0.0, 0.0, 0.4),
        (1.0, 1.0, 0.3),
        (2.0, 2.0, 0.2),
    ]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: General {
                font_size: 14,
                color: "#00FF41".to_string(),
                update_ms: 1000,
                glow_passes: default_glow_passes(),
            },
            screens: vec![
                Screen {
                    metrics: vec![
                        "cpu_usage".to_string(),
                        "ram_usage".to_string(),
                        "disk_usage".to_string(),
                        "network_details".to_string(),
                        "cpu_temp".to_string(),
                        "gpu_temp".to_string(),
                    ],
                    x_offset: 20,
                    y_offset: 20,
                }
            ],
            weather: Weather {
                lat: 51.5074,
                lon: -0.1278,
                enabled: false,
            },
            custom_files: Vec::new(),
            productivity: Productivity::default(),
            cosmetics: Cosmetics::default(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let home = env::var("HOME").context("HOME environment variable not set")?;
        let config_path = Path::new(&home).join(".config/matrix-overlay/config.json");

        if !config_path.exists() {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent).context("Failed to create config directory")?;
            }
            let default_config = Config::default();
            let json = serde_json::to_string_pretty(&default_config).context("Failed to serialize default config")?;
            fs::write(&config_path, json).context("Failed to write default config file")?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&config_path).context("Failed to read config file")?;
        let config: Config = serde_json::from_str(&content).context("Failed to parse config.json")?;

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.general.font_size < 12 {
            bail!("font_size must be >= 12");
        }
        if !self.is_valid_hex(&self.general.color) {
            bail!("color must be a valid hex string (e.g., #RRGGBB)");
        }
        if self.general.update_ms < 500 {
            bail!("update_ms must be >= 500");
        }
        for (i, screen) in self.screens.iter().enumerate() {
            if screen.x_offset < 0 || screen.y_offset < 0 {
                bail!("Screen {} offsets must be non-negative", i);
            }
        }
        Ok(())
    }

    fn is_valid_hex(&self, color: &str) -> bool {
        if !color.starts_with('#') {
            return false;
        }
        let hex = &color[1..];
        (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
    }
}

// Compatibility struct for metrics module
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
                if !config.weather.enabled && (m == "weather_temp" || m == "weather_condition") {
                    continue;
                }
                metrics.insert(m.clone());
            }
        }
        
        Self {
            refresh_rate_ms: config.general.update_ms,
            enable_nvidia: true, // Defaulting to true as it was removed from config
            active_metrics: metrics.into_iter().collect(),
            latitude: config.weather.lat,
            longitude: config.weather.lon,
        }
    }
}
