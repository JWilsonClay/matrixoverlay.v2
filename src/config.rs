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
}

fn default_metric_font_size() -> u32 { 14 }

fn default_theme() -> String { "classic".to_string() }

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

/// Productivity tracking configuration.
/// 
/// Ties to Stage 0: Productivity Features (Git/AI).
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Productivity {
    /// List of local Git repository paths to monitor.
    #[serde(default)]
    pub repos: Vec<String>,
    /// Threshold for auto-committing (not yet implemented in v2).
    #[serde(default = "default_commit_threshold")]
    pub auto_commit_threshold: u64,
    /// Whether Ollama AI insights are enabled.
    #[serde(default)]
    pub ollama_enabled: bool,
    /// Maximum number of repositories to scan per update cycle.
    #[serde(default = "default_batch_cap")]
    pub batch_cap: u32,
}

fn default_commit_threshold() -> u64 { 1000 }
fn default_batch_cap() -> u32 { 5 }

/// Cosmetic and animation configuration.
/// 
/// Ties to Stage 0: Matrix Aesthetics (<1% CPU goal).
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Cosmetics {
    /// Rain mode: "fall" (classic), "pulse" (low-resource glow), or "off".
    #[serde(default = "default_rain_mode")]
    pub rain_mode: String,
    /// Realism scale (0-10) affecting stream density and speed variance.
    #[serde(default = "default_realism")]
    pub realism_scale: u32,
    /// Whether metrics should occlude the rain for better readability.
    #[serde(default = "default_true")]
    pub occlusion_enabled: bool,
    /// rain speed multiplier (0.0 - 3.0+)
    #[serde(default = "default_rain_speed")]
    pub rain_speed: f64,
    /// Brightness for metrics (0.0 - 1.0)
    #[serde(default = "default_brightness")]
    pub metrics_brightness: f64,
    /// Brightness for matrix rain (0.0 - 1.0)
    #[serde(default = "default_brightness")]
    pub matrix_brightness: f64,
    /// Whether to draw a border around metrics
    #[serde(default)]
    pub border_enabled: bool,
    /// Border color hex
    #[serde(default = "default_border_color")]
    pub border_color: String,
    /// Opacity of the metric background box
    #[serde(default = "default_bg_opacity")]
    pub background_opacity: f64,
}

fn default_rain_speed() -> f64 { 1.0 }
fn default_brightness() -> f64 { 0.9 }
fn default_border_color() -> String { "#00FF41".to_string() }
fn default_bg_opacity() -> f64 { 0.7 }

fn default_rain_mode() -> String { "fall".to_string() }
fn default_realism() -> u32 { 10 }
fn default_true() -> bool { true }
fn default_false() -> bool { false }

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

fn default_interval() -> u64 { 30 }
fn default_max_files() -> usize { 5 }
fn default_max_size() -> u64 { 1 }

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
                metric_font_size: 14,
                color: "#00FF41".to_string(),
                update_ms: 1000, // Matching user's expected default or higher
                theme: "classic".to_string(),
                glow_passes: default_glow_passes(),
                show_monitor_label: true,
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
                lat: 0.0,
                lon: 0.0,
                enabled: false,
            },
            custom_files: Vec::new(),
            productivity: Productivity::default(),
            cosmetics: Cosmetics::default(),
            logging: Logging::default(),
        }
    }
}

impl Config {
    /// Loads configuration from `~/.config/matrix-overlay/config.json`.
    /// 
    /// If the file does not exist, it creates a default configuration.
    /// Validates the loaded configuration before returning.
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

    /// Saves configuration to `~/.config/matrix-overlay/config.json`.
    pub fn save(&self) -> Result<()> {
        let home = env::var("HOME").context("HOME environment variable not set")?;
        let config_path = Path::new(&home).join(".config/matrix-overlay/config.json");
        let json = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(config_path, json).context("Failed to write config file")?;
        Ok(())
    }

    /// Validates configuration values and safety of provided paths.
    /// 
    /// Ties to Stage 4: Security Hardening. Uses `path_utils` to verify 
    /// that all monitored files and Git repos are within safe directories.
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

        // Security Path Validation
        for file in &self.custom_files {
            if !crate::path_utils::is_safe_path(std::path::Path::new(&file.path)) {
                log::warn!("Security Warning: Unsafe path detected in custom_files: {}", file.path);
            }
        }
        for repo in &self.productivity.repos {
            if !crate::path_utils::is_safe_path(std::path::Path::new(repo)) {
                log::warn!("Security Warning: Unsafe Git repo path: {}", repo);
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
