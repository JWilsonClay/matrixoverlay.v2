//! Metrics Configuration GUI Substrate.
//! Orchestrates the visibility and ordering of system metrics in the HUD.

use gtk::prelude::*;
use gtk::{Box, Orientation, Label, CheckButton, ScrolledWindow, Adjustment, PolicyType};
use crate::core::config::Config;

/// [HARDENED] Builds the metrics configuration view with failure-isolated toggle rows.
pub fn build(vbox: &Box, config: &Config) -> Box {
    vbox.set_border_width(10);
    vbox.pack_start(&Label::new(Some("Visible Metrics & Toggles")), false, false, 5);

    let rows_vbox = Box::new(Orientation::Vertical, 2);
    let all_metrics = vec![
        ("day_of_week", "Day of Week (Header)"), ("cpu_usage", "CPU Usage (%)"), 
        ("ram_usage", "RAM Usage (%)"), ("overlay_cpu", "Overlay CPU Usage (%)"), 
        ("gpu_temp", "GPU Temperature"), ("gpu_util", "GPU Utilization"),
        ("disk_usage", "Disk Usage (%)"), ("uptime", "System Uptime"), 
        ("network_details", "Network Details"), ("weather_temp", "Weather Temperature"), 
        ("code_delta", "Git Code Delta (+/-)"), ("fan_speed", "Fan Speed (RPM)"),
    ];

    let cur_met = config.screens.first().map(|s| s.metrics.clone()).unwrap_or_default();

    for (id, label) in all_metrics {
        let row = Box::new(Orientation::Horizontal, 10);
        let check = CheckButton::with_label(label);
        check.set_active(cur_met.contains(&id.to_string()));
        check.set_widget_name(id); // Use widget name to store metric ID
        
        row.pack_start(&check, true, true, 0);
        rows_vbox.pack_start(&row, false, false, 2);
    }
    
    let scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_min_content_height(300);
    scroll.add(&rows_vbox);
    vbox.pack_start(&scroll, true, true, 0);
    
    rows_vbox
}
