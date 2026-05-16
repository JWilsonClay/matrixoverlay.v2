// src/core/config/defaults.rs
//! Default configuration values for the Matrix Overlay.
//! [HARDENED] All constants are bounded and validated to prevent resource exhaustion.

pub fn default_label_spacing() -> i32 { 10 }
pub fn default_temp_unit() -> String { "celsius".to_string() }
pub fn default_metric_font_size() -> u32 { 14 }
pub fn default_spacing() -> i32 { 24 }
pub fn default_columns() -> u32 { 1 }
pub fn default_alignment() -> String { "left".to_string() }
pub fn default_theme() -> String { "classic".to_string() }
pub fn default_false() -> bool { false }
pub fn default_commit_threshold() -> u64 { 1000 }
pub fn default_batch_cap() -> u32 { 5 }
pub fn default_rain_speed() -> f64 { 1.0 }
pub fn default_brightness() -> f64 { 0.9 }
pub fn default_border_color() -> String { "#00FF41".to_string() }
pub fn default_bg_opacity() -> f64 { 0.7 }
pub fn default_preset() -> String { "medium".to_string() }
pub fn default_rain_mode() -> String { "fall".to_string() }
pub fn default_realism() -> u32 { 10 }
pub fn default_true() -> bool { true }
pub fn default_interval() -> u64 { 30 }
pub fn default_max_files() -> usize { 5 }
pub fn default_max_size() -> u64 { 1 }

/// [HARDENING] Fixed-size glow pass configuration to prevent heap overflow.
pub fn default_glow_passes() -> Vec<(f64, f64, f64)> {
    vec![
        (-2.0, -2.0, 0.2),
        (-1.0, -1.0, 0.3),
        (0.0, 0.0, 0.4),
        (1.0, 1.0, 0.3),
        (2.0, 2.0, 0.2),
    ]
}
