//! General Configuration GUI Substrate.
//! Orchestrates theme, typography, and update interval parameters.

use gtk::prelude::*;
use gtk::{Box, Label, ComboBoxText, SpinButton, CheckButton};
use crate::core::config::Config;

/// [HARDENED] Builds the general configuration view with strictly bounded inputs.
pub fn build(vbox: &Box, config: &Config) -> (ComboBoxText, SpinButton, SpinButton, SpinButton, SpinButton, SpinButton, ComboBoxText, SpinButton, CheckButton, CheckButton, ComboBoxText, SpinButton) {
    vbox.set_border_width(10);
    
    vbox.pack_start(&Label::new(Some("Theme")), false, false, 0);
    let theme_combo = ComboBoxText::new();
    theme_combo.append_text("classic");
    theme_combo.append_text("calm");
    theme_combo.append_text("alert");
    theme_combo.set_active_id(Some(&config.general.theme));
    vbox.pack_start(&theme_combo, false, false, 0);

    vbox.pack_start(&Label::new(Some("Matrix Font Size (Rain)")), false, false, 0);
    let font_spin = SpinButton::with_range(12.0, 72.0, 1.0);
    font_spin.set_value(config.general.font_size as f64);
    vbox.pack_start(&font_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Metrics Font Size (HUD)")), false, false, 0);
    let metric_font_spin = SpinButton::with_range(8.0, 48.0, 1.0);
    metric_font_spin.set_value(config.general.metric_font_size as f64);
    vbox.pack_start(&metric_font_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Metric Vertical Spacing")), false, false, 0);
    let spacing_spin = SpinButton::with_range(10.0, 100.0, 2.0);
    spacing_spin.set_value(config.general.metric_spacing as f64);
    vbox.pack_start(&spacing_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Label-Value Spacing")), false, false, 0);
    let label_spacing_spin = SpinButton::with_range(0.0, 200.0, 2.0);
    label_spacing_spin.set_value(config.general.label_value_spacing as f64);
    vbox.pack_start(&label_spacing_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Metric Columns (1-3)")), false, false, 0);
    let columns_spin = SpinButton::with_range(1.0, 3.0, 1.0);
    columns_spin.set_value(config.general.metric_columns as f64);
    vbox.pack_start(&columns_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Block Alignment")), false, false, 0);
    let align_combo = ComboBoxText::new();
    align_combo.append_text("left");
    align_combo.append_text("center");
    align_combo.append_text("right");
    align_combo.set_active_id(Some(&config.general.metric_alignment));
    vbox.pack_start(&align_combo, false, false, 0);

    vbox.pack_start(&Label::new(Some("Update Interval (ms, min 500)")), false, false, 0);
    let update_spin = SpinButton::with_range(500.0, 10000.0, 100.0);
    update_spin.set_value(config.general.update_ms as f64);
    vbox.pack_start(&update_spin, false, false, 0);

    let check_monitor_label = CheckButton::with_label("Show Monitor Labels (e.g., Monitor 1)");
    check_monitor_label.set_active(config.general.show_monitor_label);
    vbox.pack_start(&check_monitor_label, false, false, 10);

    let check_cpu_metric = CheckButton::with_label("Show Overlay CPU Usage (Internal)");
    check_cpu_metric.set_active(config.general.show_cpu_metric);
    vbox.pack_start(&check_cpu_metric, false, false, 0);

    vbox.pack_start(&Label::new(Some("Temperature Unit")), false, false, 0);
    let temp_unit_combo = ComboBoxText::new();
    temp_unit_combo.append(Some("celsius"), "Celsius (°C)");
    temp_unit_combo.append(Some("fahrenheit"), "Fahrenheit (°F)");
    temp_unit_combo.set_active_id(Some(&config.general.temp_unit));
    vbox.pack_start(&temp_unit_combo, false, false, 0);

    // [Phase 5.5] Render rate. Range mirrors `General::fps()`'s 1..=60 clamp, so
    // the widget cannot produce a value the loader would silently change.
    vbox.pack_start(&Label::new(Some("Render Rate (fps, 1-60 — 1 is sufficient)")), false, false, 0);
    let fps_spin = SpinButton::with_range(1.0, 60.0, 1.0);
    fps_spin.set_value(config.general.fps() as f64);
    vbox.pack_start(&fps_spin, false, false, 0);

    (theme_combo, font_spin, metric_font_spin, spacing_spin, label_spacing_spin, columns_spin, align_combo, update_spin, check_monitor_label, check_cpu_metric, temp_unit_combo, fps_spin)
}
