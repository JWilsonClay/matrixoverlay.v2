// src/ui/gui/logic.rs
use crate::core::config::Config;
use super::metrics;

pub fn update_config_from_widgets(
    new_config: &mut Config,
    gen_w: &(gtk::ComboBoxText, gtk::SpinButton, gtk::SpinButton, gtk::SpinButton, gtk::SpinButton, gtk::SpinButton, gtk::ComboBoxText, gtk::SpinButton, gtk::CheckButton, gtk::CheckButton, gtk::ComboBoxText, gtk::SpinButton),
    cos_w: &(gtk::SpinButton, gtk::SpinButton, gtk::SpinButton, gtk::SpinButton, gtk::SpinButton, gtk::CheckButton, gtk::CheckButton),
    weath_w: &(gtk::CheckButton, gtk::CheckButton, gtk::SpinButton, gtk::SpinButton),
) {
    use gtk::prelude::*;
    // General
    if let Some(id) = gen_w.0.active_id() { new_config.general.theme = id.to_string(); }
    new_config.general.font_size = gen_w.1.value() as u32;
    new_config.general.metric_font_size = gen_w.2.value() as u32;
    new_config.general.metric_spacing = gen_w.3.value() as i32;
    new_config.general.label_value_spacing = gen_w.4.value() as i32;
    new_config.general.metric_columns = gen_w.5.value() as u32;
    if let Some(id) = gen_w.6.active_id() { new_config.general.metric_alignment = id.to_string(); }
    new_config.general.update_ms = gen_w.7.value() as u64;
    new_config.general.show_monitor_label = gen_w.8.is_active();
    new_config.general.show_cpu_metric = gen_w.9.is_active();
    if let Some(id) = gen_w.10.active_id() { new_config.general.temp_unit = id.to_string(); }
    new_config.general.target_fps = gen_w.11.value() as u32;

    // Cosmetics
    new_config.cosmetics.rain_speed = cos_w.0.value();
    new_config.cosmetics.realism = cos_w.1.value() as u32;
    new_config.cosmetics.metrics_brightness = cos_w.2.value();
    new_config.cosmetics.matrix_brightness = cos_w.3.value();
    new_config.cosmetics.background_opacity = cos_w.4.value();
    new_config.cosmetics.occlusion_enabled = cos_w.5.is_active();
    new_config.cosmetics.border_enabled = cos_w.6.is_active();

    // Weather
    new_config.weather.enabled = weath_w.0.is_active();
    new_config.weather.auto_location = weath_w.1.is_active();
    new_config.weather.lat = weath_w.2.value();
    new_config.weather.lon = weath_w.3.value();
}
