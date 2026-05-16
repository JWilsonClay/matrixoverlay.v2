// src/ui/gui/cosmetics.rs
use gtk::prelude::*;
use gtk::{Box, Label, SpinButton, CheckButton};
use crate::core::config::Config;

pub fn build(vbox: &Box, config: &Config) -> (SpinButton, SpinButton, SpinButton, SpinButton, SpinButton, CheckButton, CheckButton) {
    vbox.set_border_width(10);

    vbox.pack_start(&Label::new(Some("Rain Speed Multiplier")), false, false, 0);
    let speed_spin = SpinButton::with_range(0.0, 5.0, 0.1);
    speed_spin.set_value(config.cosmetics.rain_speed);
    vbox.pack_start(&speed_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Rain Density (Volume)")), false, false, 0);
    let realism_spin = SpinButton::with_range(0.0, 50.0, 1.0);
    realism_spin.set_value(config.cosmetics.realism_scale as f64);
    vbox.pack_start(&realism_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Metrics Brightness (HUD)")), false, false, 0);
    let metrics_bright_spin = SpinButton::with_range(0.0, 1.0, 0.05);
    metrics_bright_spin.set_value(config.cosmetics.metrics_brightness);
    vbox.pack_start(&metrics_bright_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Matrix Brightness (Rain)")), false, false, 0);
    let matrix_bright_spin = SpinButton::with_range(0.0, 1.0, 0.05);
    matrix_bright_spin.set_value(config.cosmetics.matrix_brightness);
    vbox.pack_start(&matrix_bright_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Background Opacity")), false, false, 0);
    let opac_spin = SpinButton::with_range(0.0, 1.0, 0.05);
    opac_spin.set_value(config.cosmetics.background_opacity);
    vbox.pack_start(&opac_spin, false, false, 0);

    let check_occlusion = CheckButton::with_label("Enable Occlusion (Rain behind metrics)");
    check_occlusion.set_active(config.cosmetics.occlusion_enabled);
    vbox.pack_start(&check_occlusion, false, false, 0);

    let check_border = CheckButton::with_label("Metric HUD Borders");
    check_border.set_active(config.cosmetics.border_enabled);
    vbox.pack_start(&check_border, false, false, 0);

    (speed_spin, realism_spin, metrics_bright_spin, matrix_bright_spin, opac_spin, check_occlusion, check_border)
}
