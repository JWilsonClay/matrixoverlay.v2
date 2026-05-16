// src/core/config/types.rs
use serde::{Deserialize, Serialize};
use super::defaults::*;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Screen {
    pub metrics: Vec<String>,
    pub x_offset: i32,
    pub y_offset: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Weather {
    pub lat: f64,
    pub lon: f64,
    pub enabled: bool,
    #[serde(default = "default_false")]
    pub auto_location: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CustomFile {
    pub name: String,
    pub path: String,
    pub metric_id: String,
    #[serde(default)]
    pub tail: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct Cosmetics {
    #[serde(default = "default_rain_mode")]
    pub rain_mode: String,
    #[serde(default = "default_realism")]
    pub realism: u32,
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
            realism: default_realism(),
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
#[serde(deny_unknown_fields)]
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
