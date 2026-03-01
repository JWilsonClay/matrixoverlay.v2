use gtk::prelude::*;
use gtk::{Window, WindowType, Notebook, Box, Orientation, Label, CheckButton, SpinButton, ComboBoxText, Button, Entry, ListBox};
use std::sync::Arc;
use crossbeam_channel::Sender;
use crate::config::Config;

pub enum GuiEvent {
    Reload,
    PurgeLogs,
    OpenConfig,
}

pub struct ConfigWindow {
    config: Arc<Config>,
    event_tx: Sender<GuiEvent>,
}

impl ConfigWindow {
    pub fn new(config: Config, event_tx: Sender<GuiEvent>) -> Self {
        Self {
            config: Arc::new(config),
            event_tx,
        }
    }

    pub fn show(&self) {
        let window = Window::new(WindowType::Toplevel);
        window.set_title("Matrix Overlay v2 - Configuration");
        window.set_default_size(500, 750);

        let notebook = Notebook::new();
        
        // --- 1. General Tab ---
        let vbox_gen = Box::new(Orientation::Vertical, 10);
        vbox_gen.set_border_width(10);
        vbox_gen.pack_start(&Label::new(Some("Theme")), false, false, 0);
        let theme_combo = ComboBoxText::new();
        theme_combo.append_text("classic");
        theme_combo.append_text("calm");
        theme_combo.append_text("alert");
        theme_combo.set_active_id(Some(&self.config.general.theme));
        vbox_gen.pack_start(&theme_combo, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Matrix Font Size (Rain)")), false, false, 0);
        let font_spin = SpinButton::with_range(12.0, 72.0, 1.0);
        font_spin.set_value(self.config.general.font_size as f64);
        vbox_gen.pack_start(&font_spin, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Metrics Font Size (HUD)")), false, false, 0);
        let metric_font_spin = SpinButton::with_range(8.0, 48.0, 1.0);
        metric_font_spin.set_value(self.config.general.metric_font_size as f64);
        vbox_gen.pack_start(&metric_font_spin, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Update Interval (ms, min 500)")), false, false, 0);
        let update_spin = SpinButton::with_range(500.0, 10000.0, 100.0);
        update_spin.set_value(self.config.general.update_ms as f64);
        vbox_gen.pack_start(&update_spin, false, false, 0);

        let check_monitor_label = CheckButton::with_label("Show Monitor Labels (e.g., Monitor 1)");
        check_monitor_label.set_active(self.config.general.show_monitor_label);
        vbox_gen.pack_start(&check_monitor_label, false, false, 0);

        notebook.append_page(&vbox_gen, Some(&Label::new(Some("General"))));

        // --- 2. Metrics Tab (REORDERABLE) ---
        let vbox_met = Box::new(Orientation::Vertical, 5);
        vbox_met.set_border_width(10);
        vbox_met.pack_start(&Label::new(Some("Visible Metrics & Order")), false, false, 5);
        
        let listbox = ListBox::new();
        listbox.set_selection_mode(gtk::SelectionMode::None);

        let all_metrics = vec![
            ("cpu_usage", "CPU Usage (%)"),
            ("ram_usage", "RAM Usage (%)"),
            ("gpu_temp", "GPU Temperature"),
            ("gpu_util", "GPU Utilization"),
            ("disk_usage", "Disk Usage (%)"),
            ("uptime", "System Uptime"),
            ("network_details", "Network Details"),
            ("weather_temp", "Weather Temperature"),
            ("code_delta", "Git Code Delta (+/-)"),
            ("fan_speed", "Fan Speed (RPM)"),
        ];

        // Current order from config, followed by any missing ones
        let mut current_metrics = self.config.screens.first().map(|s| s.metrics.clone()).unwrap_or_default();
        // Filter out day_of_week as it's the header
        current_metrics.retain(|m| m != "day_of_week" && m != "weather_condition");

        for (id, _) in &all_metrics {
            if !current_metrics.contains(&id.to_string()) {
                // We don't add weather_condition manually, it's tied to weather_temp
                if *id != "weather_condition" {
                    // current_metrics.push(id.to_string()); // Don't auto-add, just show what's in config
                }
            }
        }

        // We need a shared state for the list rows
        let rows_vbox = Box::new(Orientation::Vertical, 2);
        
        let create_row = |id: String, label: String, active: bool| -> Box {
             let row_box = Box::new(Orientation::Horizontal, 5);
             let check = CheckButton::with_label(&label);
             check.set_active(active);
             check.set_widget_name(&id); // Store ID in name for retrieval
             
             row_box.pack_start(&check, true, true, 0);
             
             let btn_up = Button::with_label("↑");
             let btn_down = Button::with_label("↓");
             
             row_box.pack_start(&btn_up, false, false, 0);
             row_box.pack_start(&btn_down, false, false, 0);
             
             // Move Logic
             let row_box_clone = row_box.clone();
             let rows_vbox_clone = rows_vbox.clone();
             btn_up.connect_clicked(move |_| {
                 let parent = rows_vbox_clone.clone();
                 let pos = parent.children().iter().position(|c| c == &row_box_clone).unwrap_or(0);
                 if pos > 0 {
                     parent.reorder_child(&row_box_clone, (pos - 1) as i32);
                 }
             });
             
             let row_box_clone2 = row_box.clone();
             let rows_vbox_clone2 = rows_vbox.clone();
             btn_down.connect_clicked(move |_| {
                 let parent = rows_vbox_clone2.clone();
                 let children = parent.children();
                 let pos = children.iter().position(|c| c == &row_box_clone2).unwrap_or(0);
                 if pos < children.len() - 1 {
                     parent.reorder_child(&row_box_clone2, (pos + 1) as i32);
                 }
             });

             row_box
        };

        // Add already active ones in order
        for id in &current_metrics {
            if let Some((_, label)) = all_metrics.iter().find(|(mid, _)| mid == id) {
                rows_vbox.pack_start(&create_row(id.clone(), label.to_string(), true), false, false, 0);
            }
        }
        
        // Add inactive ones
        for (id, label) in &all_metrics {
            if !current_metrics.contains(&id.to_string()) {
                rows_vbox.pack_start(&create_row(id.to_string(), label.to_string(), false), false, false, 0);
            }
        }

        vbox_met.pack_start(&rows_vbox, false, false, 0);
        notebook.append_page(&vbox_met, Some(&Label::new(Some("Metrics"))));

        // --- 3. Cosmetics Tab ---
        let vbox_cos = Box::new(Orientation::Vertical, 10);
        vbox_cos.set_border_width(10);

        vbox_cos.pack_start(&Label::new(Some("Rain Speed Multiplier (0.0 = static fade)")), false, false, 0);
        let speed_spin = SpinButton::with_range(0.0, 5.0, 0.1);
        speed_spin.set_value(self.config.cosmetics.rain_speed);
        vbox_cos.pack_start(&speed_spin, false, false, 0);

        vbox_cos.pack_start(&Label::new(Some("Rain Density (Volume, 0-50, 50=Realism)")), false, false, 0);
        let realism_spin = SpinButton::with_range(0.0, 50.0, 1.0);
        realism_spin.set_value(self.config.cosmetics.realism_scale as f64);
        vbox_cos.pack_start(&realism_spin, false, false, 0);

        vbox_cos.pack_start(&Label::new(Some("Metrics Brightness (HUD)")), false, false, 0);
        let metrics_bright_spin = SpinButton::with_range(0.0, 1.0, 0.05);
        metrics_bright_spin.set_value(self.config.cosmetics.metrics_brightness);
        vbox_cos.pack_start(&metrics_bright_spin, false, false, 0);

        vbox_cos.pack_start(&Label::new(Some("Matrix Brightness (Rain)")), false, false, 0);
        let matrix_bright_spin = SpinButton::with_range(0.0, 1.0, 0.05);
        matrix_bright_spin.set_value(self.config.cosmetics.matrix_brightness);
        vbox_cos.pack_start(&matrix_bright_spin, false, false, 0);

        vbox_cos.pack_start(&Label::new(Some("Background Opacity")), false, false, 0);
        let opac_spin = SpinButton::with_range(0.0, 1.0, 0.05);
        opac_spin.set_value(self.config.cosmetics.background_opacity);
        vbox_cos.pack_start(&opac_spin, false, false, 0);

        vbox_cos.pack_start(&Label::new(Some("Occlusion Options")), false, false, 0);
        let check_occlusion = CheckButton::with_label("Enable Occlusion (Rain behind metrics)");
        check_occlusion.set_active(self.config.cosmetics.occlusion_enabled);
        vbox_cos.pack_start(&check_occlusion, false, false, 0);

        let check_border = CheckButton::with_label("Metric HUD Borders");
        check_border.set_active(self.config.cosmetics.border_enabled);
        vbox_cos.pack_start(&check_border, false, false, 0);

        notebook.append_page(&vbox_cos, Some(&Label::new(Some("Cosmetics"))));

        // --- 4. Productivity Tab ---
        let vbox_prod = Box::new(Orientation::Vertical, 10);
        vbox_prod.set_border_width(10);

        let check_ollama = CheckButton::with_label("Enable Ollama AI Insights");
        check_ollama.set_active(self.config.productivity.ollama_enabled);
        vbox_prod.pack_start(&check_ollama, false, false, 0);

        vbox_prod.pack_start(&Label::new(Some("Git Repositories (Comma separated)")), false, false, 0);
        let repos_entry = Entry::new();
        repos_entry.set_text(&self.config.productivity.repos.join(", "));
        vbox_prod.pack_start(&repos_entry, false, false, 0);

        vbox_prod.pack_start(&Label::new(Some("Auto-Commit Threshold (Lines)")), false, false, 0);
        let commit_spin = SpinButton::with_range(0.0, 5000.0, 100.0);
        commit_spin.set_value(self.config.productivity.auto_commit_threshold as f64);
        vbox_prod.pack_start(&commit_spin, false, false, 0);

        notebook.append_page(&vbox_prod, Some(&Label::new(Some("Productivity"))));

        // --- 5. Weather Tab ---
        let vbox_weath = Box::new(Orientation::Vertical, 10);
        vbox_weath.set_border_width(10);

        let check_weather_enabled = CheckButton::with_label("Enable Weather Data (Open-Meteo)");
        check_weather_enabled.set_active(self.config.weather.enabled);
        vbox_weath.pack_start(&check_weather_enabled, false, false, 0);

        vbox_weath.pack_start(&Label::new(Some("Location (0.0/0.0 triggers Geo-IP Auto)")), false, false, 0);
        vbox_weath.pack_start(&Label::new(Some("Latitude")), false, false, 0);
        let lat_spin = SpinButton::with_range(-90.0, 90.0, 0.0001);
        lat_spin.set_value(self.config.weather.lat);
        vbox_weath.pack_start(&lat_spin, false, false, 0);

        vbox_weath.pack_start(&Label::new(Some("Longitude")), false, false, 0);
        let lon_spin = SpinButton::with_range(-180.0, 180.0, 0.0001);
        lon_spin.set_value(self.config.weather.lon);
        vbox_weath.pack_start(&lon_spin, false, false, 0);

        notebook.append_page(&vbox_weath, Some(&Label::new(Some("Weather"))));

        // --- 6. Advanced Tab ---
        let vbox_adv = Box::new(Orientation::Vertical, 10);
        vbox_adv.set_border_width(10);
        
        vbox_adv.pack_start(&Label::new(Some("Debug & Maintenance")), false, false, 0);
        let btn_purge = Button::with_label("Purge Debug Logs (/tmp)");
        vbox_adv.pack_start(&btn_purge, false, false, 0);
        
        notebook.append_page(&vbox_adv, Some(&Label::new(Some("Advanced"))));

        // --- Bottom Actions ---
        let main_vbox = Box::new(Orientation::Vertical, 10);
        main_vbox.pack_start(&notebook, true, true, 5);

        let hbox = Box::new(Orientation::Horizontal, 10);
        let btn_cancel = Button::with_label("Cancel");
        let btn_save = Button::with_label("Save & Apply Changes");
        hbox.pack_end(&btn_save, false, false, 5);
        hbox.pack_end(&btn_cancel, false, false, 5);
        main_vbox.pack_start(&hbox, false, false, 10);

        // Wiring logic
        let tx = self.event_tx.clone();
        let config_arc = self.config.clone();
        btn_save.connect_clicked(move |_| {
            let mut new_config = (*config_arc).clone();
            
            // General
            new_config.general.theme = theme_combo.active_text().map(|s| s.to_string()).unwrap_or_else(|| "classic".to_string());
            new_config.general.font_size = font_spin.value() as u32;
            new_config.general.metric_font_size = metric_font_spin.value() as u32;
            new_config.general.update_ms = update_spin.value() as u64;
            new_config.general.show_monitor_label = check_monitor_label.is_active();
            
            // Cosmetics
            new_config.cosmetics.rain_speed = speed_spin.value();
            new_config.cosmetics.realism_scale = realism_spin.value() as u32;
            new_config.cosmetics.metrics_brightness = metrics_bright_spin.value();
            new_config.cosmetics.matrix_brightness = matrix_bright_spin.value();
            new_config.cosmetics.background_opacity = opac_spin.value();
            new_config.cosmetics.occlusion_enabled = check_occlusion.is_active();
            new_config.cosmetics.border_enabled = check_border.is_active();

            // Productivity
            new_config.productivity.ollama_enabled = check_ollama.is_active();
            new_config.productivity.auto_commit_threshold = commit_spin.value() as u64;
            new_config.productivity.repos = repos_entry.text().to_string()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            // Weather
            new_config.weather.enabled = check_weather_enabled.is_active();
            new_config.weather.lat = lat_spin.value();
            new_config.weather.lon = lon_spin.value();

            // Metrics Selection & Order (Extracted from UI order)
            let mut active_metrics = Vec::new();
            active_metrics.push("day_of_week".to_string()); // Always first
            
            for row in rows_vbox.children() {
                if let Some(row_box) = row.downcast_ref::<Box>() {
                    if let Some(check) = row_box.children().first().and_then(|c| c.downcast_ref::<CheckButton>()) {
                        if check.is_active() {
                            let id = check.widget_name().to_string();
                            active_metrics.push(id.clone());
                            // Special case: condition tied to temp
                            if id == "weather_temp" {
                                active_metrics.push("weather_condition".to_string());
                            }
                        }
                    }
                }
            }

            for screen in &mut new_config.screens {
                screen.metrics = active_metrics.clone();
            }

            if let Err(e) = new_config.save() {
                log::error!("Failed to save config: {}", e);
            }
            let _ = tx.send(GuiEvent::Reload);
        });

        let tx_purge = self.event_tx.clone();
        btn_purge.connect_clicked(move |_| {
            let _ = tx_purge.send(GuiEvent::PurgeLogs);
        });

        let win_cancel = window.clone();
        btn_cancel.connect_clicked(move |_| {
            win_cancel.close();
        });

        window.add(&main_vbox);
        window.show_all();
    }
}
