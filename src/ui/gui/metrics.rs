// src/ui/gui/metrics.rs
use gtk::prelude::*;
use gtk::{Box, Orientation, Label, CheckButton, Button, ScrolledWindow, Adjustment, PolicyType};
use crate::core::config::Config;

pub fn build(vbox: &Box, config: &Config, trigger_save: impl Fn() + Clone + 'static) -> Box {
    vbox.set_border_width(10);
    vbox.pack_start(&Label::new(Some("Visible Metrics & Order")), false, false, 5);

    let rows_vbox = Box::new(Orientation::Vertical, 2);
    let all_metrics = vec![
        ("day_of_week", "Day of Week (Header)"), ("cpu_usage", "CPU Usage (%)"), ("ram_usage", "RAM Usage (%)"),
        ("overlay_cpu", "Overlay CPU Usage (%)"), ("gpu_temp", "GPU Temperature"), ("gpu_util", "GPU Utilization"),
        ("disk_usage", "Disk Usage (%)"), ("uptime", "System Uptime"), ("network_details", "Network Details"),
        ("weather_temp", "Weather Temperature"), ("code_delta", "Git Code Delta (+/-)"), ("fan_speed", "Fan Speed (RPM)"),
    ];

    let mut cur_met = config.screens.first().map(|s| s.metrics.clone()).unwrap_or_default();
    cur_met.retain(|m| m != "weather_condition");

    let rv_copy = rows_vbox.clone();
    let ts_row = trigger_save;

    let create_row = move |id: String, label: String, active: bool, rv: Box, ts: Box| -> Box {
         // row creation logic... 
         // Actually I'll simplify the signature for the extraction
         Box::new(Orientation::Horizontal, 0)
    };
    
    // I'll implement the full logic in mod.rs or here once I finalize the structure.
    // For now, I'll just return the rows_vbox in a scrollable container.
    
    let scroll = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.add(&rows_vbox);
    vbox.pack_start(&scroll, true, true, 0);
    
    rows_vbox
}
