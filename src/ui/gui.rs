use gtk::prelude::*;
use gtk::{Window, WindowType, Notebook, Box, Orientation, Label, CheckButton, SpinButton, ComboBoxText, Button, Entry, ScrolledWindow, Adjustment, PolicyType};
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
        window.set_default_size(550, 850);

        let notebook = Notebook::new();
        let rows_vbox = Box::new(Orientation::Vertical, 2);
        
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

        vbox_gen.pack_start(&Label::new(Some("Metric Vertical Spacing")), false, false, 0);
        let spacing_spin = SpinButton::with_range(10.0, 100.0, 2.0);
        spacing_spin.set_value(self.config.general.metric_spacing as f64);
        vbox_gen.pack_start(&spacing_spin, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Label-Value Spacing")), false, false, 0);
        let label_spacing_spin = SpinButton::with_range(0.0, 200.0, 2.0);
        label_spacing_spin.set_value(self.config.general.label_value_spacing as f64);
        vbox_gen.pack_start(&label_spacing_spin, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Metric Columns (1-3)")), false, false, 0);
        let columns_spin = SpinButton::with_range(1.0, 3.0, 1.0);
        columns_spin.set_value(self.config.general.metric_columns as f64);
        vbox_gen.pack_start(&columns_spin, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Block Alignment")), false, false, 0);
        let align_combo = ComboBoxText::new();
        align_combo.append_text("left");
        align_combo.append_text("center");
        align_combo.append_text("right");
        align_combo.set_active_id(Some(&self.config.general.metric_alignment));
        vbox_gen.pack_start(&align_combo, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Update Interval (ms, min 500)")), false, false, 0);
        let update_spin = SpinButton::with_range(500.0, 10000.0, 100.0);
        update_spin.set_value(self.config.general.update_ms as f64);
        vbox_gen.pack_start(&update_spin, false, false, 0);

        let check_monitor_label = CheckButton::with_label("Show Monitor Labels (e.g., Monitor 1)");
        check_monitor_label.set_active(self.config.general.show_monitor_label);
        vbox_gen.pack_start(&check_monitor_label, false, false, 0);

        let check_cpu_metric = CheckButton::with_label("Show Overlay CPU Usage (Internal)");
        check_cpu_metric.set_active(self.config.general.show_cpu_metric);
        vbox_gen.pack_start(&check_cpu_metric, false, false, 0);

        vbox_gen.pack_start(&Label::new(Some("Temperature Unit")), false, false, 0);
        let temp_unit_combo = ComboBoxText::new();
        temp_unit_combo.append(Some("celsius"), "Celsius (°C)");
        temp_unit_combo.append(Some("fahrenheit"), "Fahrenheit (°F)");
        temp_unit_combo.set_active_id(Some(&self.config.general.temp_unit));
        vbox_gen.pack_start(&temp_unit_combo, false, false, 0);

        notebook.append_page(&vbox_gen, Some(&Label::new(Some("General"))));

        // --- 2. Cosmetics Tab ---
        let vbox_cos = Box::new(Orientation::Vertical, 10);
        vbox_cos.set_border_width(10);

        vbox_cos.pack_start(&Label::new(Some("Rain Speed Multiplier")), false, false, 0);
        let speed_spin = SpinButton::with_range(0.0, 5.0, 0.1);
        speed_spin.set_value(self.config.cosmetics.rain_speed);
        vbox_cos.pack_start(&speed_spin, false, false, 0);

        vbox_cos.pack_start(&Label::new(Some("Rain Density (Volume)")), false, false, 0);
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

        let check_occlusion = CheckButton::with_label("Enable Occlusion (Rain behind metrics)");
        check_occlusion.set_active(self.config.cosmetics.occlusion_enabled);
        vbox_cos.pack_start(&check_occlusion, false, false, 0);

        let check_border = CheckButton::with_label("Metric HUD Borders");
        check_border.set_active(self.config.cosmetics.border_enabled);
        vbox_cos.pack_start(&check_border, false, false, 0);

        notebook.append_page(&vbox_cos, Some(&Label::new(Some("Cosmetics"))));

        // --- 3. Productivity Tab ---
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

        // --- 4. Weather Tab ---
        let vbox_weath = Box::new(Orientation::Vertical, 10);
        vbox_weath.set_border_width(10);

        let check_weather_enabled = CheckButton::with_label("Enable Weather Data (Open-Meteo)");
        check_weather_enabled.set_active(self.config.weather.enabled);
        vbox_weath.pack_start(&check_weather_enabled, false, false, 0);

        let check_auto_loc = CheckButton::with_label("Automatic Location (Geo-IP)");
        check_auto_loc.set_active(self.config.weather.auto_location);
        vbox_weath.pack_start(&check_auto_loc, false, false, 0);

        vbox_weath.pack_start(&Label::new(Some("Manual Latitude")), false, false, 0);
        let lat_spin = SpinButton::with_range(-90.0, 90.0, 0.0001);
        lat_spin.set_value(self.config.weather.lat);
        vbox_weath.pack_start(&lat_spin, false, false, 0);

        vbox_weath.pack_start(&Label::new(Some("Manual Longitude")), false, false, 0);
        let lon_spin = SpinButton::with_range(-180.0, 180.0, 0.0001);
        lon_spin.set_value(self.config.weather.lon);
        vbox_weath.pack_start(&lon_spin, false, false, 0);

        // Interaction logic
        let update_weather_sensitivity = {
            let cal = check_auto_loc.clone();
            let ls = lat_spin.clone();
            let lns = lon_spin.clone();
            move |enabled: bool| {
                cal.set_sensitive(enabled);
                let auto = cal.is_active();
                ls.set_sensitive(enabled && !auto);
                lns.set_sensitive(enabled && !auto);
            }
        };
        update_weather_sensitivity(self.config.weather.enabled);
        let uws = update_weather_sensitivity.clone();
        check_weather_enabled.connect_toggled(move |cb| uws(cb.is_active()));
        let lsc = lat_spin.clone(); let lnsc = lon_spin.clone(); let cwec = check_weather_enabled.clone();
        check_auto_loc.connect_toggled(move |cb| {
            let enabled = cwec.is_active();
            let auto = cb.is_active();
            lsc.set_sensitive(enabled && !auto);
            lnsc.set_sensitive(enabled && !auto);

            if enabled && auto {
                let ls = lsc.clone();
                let lns = lnsc.clone();
                let (tx, rx) = gtk::glib::MainContext::channel(gtk::glib::Priority::default());
                
                rx.attach(None, move |(lat, lon)| {
                    ls.set_value(lat);
                    lns.set_value(lon);
                    gtk::glib::Continue(false)
                });

                std::thread::spawn(move || {
                    if let Ok((lat, lon)) = crate::metrics::fetch_geoip_location() {
                         let _ = tx.send((lat, lon));
                    }
                });
            }
        });

        notebook.append_page(&vbox_weath, Some(&Label::new(Some("Weather"))));

        // --- 5. Advanced Tab ---
        let vbox_adv = Box::new(Orientation::Vertical, 10);
        vbox_adv.set_border_width(10);
        let btn_min = Button::with_label("Minimal");
        let btn_med = Button::with_label("Medium");
        let btn_ext = Button::with_label("Extreme");
        let hbox_presets = Box::new(Orientation::Horizontal, 5);
        hbox_presets.pack_start(&btn_min, true, true, 0);
        hbox_presets.pack_start(&btn_med, true, true, 0);
        hbox_presets.pack_start(&btn_ext, true, true, 0);
        vbox_adv.pack_start(&Label::new(Some("Performance Presets")), false, false, 0);
        vbox_adv.pack_start(&hbox_presets, false, false, 0);

        let check_logging = CheckButton::with_label("Enable Debug Logging (Auto-rotated)");
        check_logging.set_active(self.config.logging.enabled);
        vbox_adv.pack_start(&check_logging, false, false, 0);

        let btn_purge = Button::with_label("Purge Debug Logs (/tmp)");
        vbox_adv.pack_start(&Label::new(Some("Maintenance")), false, false, 5);
        vbox_adv.pack_start(&btn_purge, false, false, 0);
        notebook.append_page(&vbox_adv, Some(&Label::new(Some("Advanced"))));

        // --- Bottom Actions ---
        let main_vbox = Box::new(Orientation::Vertical, 10);
        main_vbox.pack_start(&notebook, true, true, 5);
        let hbox_btns = Box::new(Orientation::Horizontal, 10);
        let btn_cancel = Button::with_label("Cancel");
        let btn_save = Button::with_label("Save & Apply Changes");
        hbox_btns.pack_end(&btn_save, false, false, 5);
        hbox_btns.pack_end(&btn_cancel, false, false, 5);
        main_vbox.pack_start(&hbox_btns, false, false, 10);

        // --- Helper for Live Updates ---
        let trigger_save = {
            let tx = self.event_tx.clone();
            let cfg_arc = self.config.clone();
            let tc = theme_combo.clone(); let fs = font_spin.clone(); let mfs = metric_font_spin.clone();
            let ms = spacing_spin.clone(); let lvs = label_spacing_spin.clone(); let mc = columns_spin.clone();
            let ma = align_combo.clone(); let ui = update_spin.clone(); let sml = check_monitor_label.clone();
            let scm = check_cpu_metric.clone(); let tuc = temp_unit_combo.clone(); let ssp = speed_spin.clone(); let rsc = realism_spin.clone();
            let mbs = metrics_bright_spin.clone(); let mxs = matrix_bright_spin.clone(); let bop = opac_spin.clone();
            let oec = check_occlusion.clone(); let bec = check_border.clone(); let coc = check_ollama.clone();
            let act = commit_spin.clone(); let rep = repos_entry.clone(); let wec = check_weather_enabled.clone();
            let alc = check_auto_loc.clone(); let lat = lat_spin.clone(); let lon = lon_spin.clone();
            let clog = check_logging.clone();
            let rvc = rows_vbox.clone();

            move || {
                let mut nc = (*cfg_arc).clone();
                nc.general.theme = tc.active_text().map(|s| s.to_string()).unwrap_or("classic".to_string());
                nc.general.font_size = fs.value() as u32;
                nc.general.metric_font_size = mfs.value() as u32;
                nc.general.metric_spacing = ms.value() as i32;
                nc.general.label_value_spacing = lvs.value() as i32;
                nc.general.metric_columns = mc.value() as u32;
                nc.general.metric_alignment = ma.active_text().map(|s| s.to_string()).unwrap_or("left".to_string());
                nc.general.update_ms = ui.value() as u64;
                nc.general.show_monitor_label = sml.is_active();
                nc.general.show_cpu_metric = scm.is_active();
                nc.general.temp_unit = tuc.active_id().map(|s| s.to_string()).unwrap_or("celsius".to_string());
                nc.cosmetics.rain_speed = ssp.value();
                nc.cosmetics.realism_scale = rsc.value() as u32;
                nc.cosmetics.metrics_brightness = mbs.value();
                nc.cosmetics.matrix_brightness = mxs.value();
                nc.cosmetics.background_opacity = bop.value();
                nc.cosmetics.occlusion_enabled = oec.is_active();
                nc.cosmetics.border_enabled = bec.is_active();
                nc.productivity.ollama_enabled = coc.is_active();
                nc.productivity.auto_commit_threshold = act.value() as u64;
                nc.productivity.repos = rep.text().to_string().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                nc.weather.enabled = wec.is_active();
                nc.weather.auto_location = alc.is_active();
                nc.weather.lat = lat.value();
                nc.weather.lon = lon.value();
                nc.logging.enabled = clog.is_active();

                let mut metrics = Vec::new();
                for row in rvc.children() {
                    if let Some(row_box) = row.downcast_ref::<Box>() {
                        if let Some(check) = row_box.children().first().and_then(|c| c.downcast_ref::<CheckButton>()) {
                            if check.is_active() {
                                let id = check.widget_name().to_string();
                                metrics.push(id.clone());
                                if id == "weather_temp" { metrics.push("weather_condition".to_string()); }
                            }
                        }
                    }
                }
                for s in &mut nc.screens { s.metrics = metrics.clone(); }
                if let Err(e) = nc.save() { log::error!("Failed to save config: {}", e); }
                let _ = tx.send(GuiEvent::Reload);
            }
        };

        // --- Metric Tab Logic (uses trigger_save) ---
        let all_metrics = vec![
            ("day_of_week", "Day of Week (Header)"), ("cpu_usage", "CPU Usage (%)"), ("ram_usage", "RAM Usage (%)"),
            ("overlay_cpu", "Overlay CPU Usage (%)"), ("gpu_temp", "GPU Temperature"), ("gpu_util", "GPU Utilization"),
            ("disk_usage", "Disk Usage (%)"), ("uptime", "System Uptime"), ("network_details", "Network Details"),
            ("weather_temp", "Weather Temperature"), ("code_delta", "Git Code Delta (+/-)"), ("fan_speed", "Fan Speed (RPM)"),
        ];
        let mut cur_met = self.config.screens.first().map(|s| s.metrics.clone()).unwrap_or_default();
        cur_met.retain(|m| m != "weather_condition");

        let ts_row = trigger_save.clone();
        let rv_copy = rows_vbox.clone();
        let create_row = move |id: String, label: String, active: bool| -> Box {
             let row_box = Box::new(Orientation::Horizontal, 5);
             let check = CheckButton::with_label(&label);
             check.set_active(active);
             check.set_widget_name(&id);
             let ts_c = ts_row.clone();
             check.connect_toggled(move |_| ts_c());
             row_box.pack_start(&check, true, true, 0);
             let btn_up = Button::with_label("↑"); let btn_down = Button::with_label("↓");
             row_box.pack_start(&btn_up, false, false, 0); row_box.pack_start(&btn_down, false, false, 0);
             let rb_u = row_box.clone(); let rv_u = rv_copy.clone(); let ts_u = ts_row.clone();
             btn_up.connect_clicked(move |_| {
                 let pos = rv_u.children().iter().position(|c| c == &rb_u).unwrap_or(0);
                 if pos > 0 { rv_u.reorder_child(&rb_u, (pos - 1) as i32); ts_u(); }
             });
             let rb_d = row_box.clone(); let rv_d = rv_copy.clone(); let ts_d = ts_row.clone();
             btn_down.connect_clicked(move |_| {
                 let children = rv_d.children();
                 let pos = children.iter().position(|c| c == &rb_d).unwrap_or(0);
                 if pos < children.len() - 1 { rv_d.reorder_child(&rb_d, (pos + 1) as i32); ts_d(); }
             });
             row_box
        };

        for id in &cur_met {
            if let Some((_, label)) = all_metrics.iter().find(|(mid, _)| mid == id) {
                rows_vbox.pack_start(&create_row(id.clone(), label.to_string(), true), false, false, 0);
            }
        }
        for (id, label) in &all_metrics {
            if !cur_met.contains(&id.to_string()) {
                rows_vbox.pack_start(&create_row(id.to_string(), label.to_string(), false), false, false, 0);
            }
        }
        let vbox_met = Box::new(Orientation::Vertical, 5);
        vbox_met.set_border_width(10);
        vbox_met.pack_start(&Label::new(Some("Visible Metrics & Order")), false, false, 5);
        let scroll_met = ScrolledWindow::new(None::<&Adjustment>, None::<&Adjustment>);
        scroll_met.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll_met.add(&rows_vbox);
        vbox_met.pack_start(&scroll_met, true, true, 0);
        notebook.insert_page(&vbox_met, Some(&Label::new(Some("Metrics"))), Some(1));

        // --- Final Connections ---
        let ts = trigger_save.clone(); theme_combo.connect_changed(move |_| ts());
        let ts = trigger_save.clone(); font_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); metric_font_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); spacing_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); label_spacing_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); columns_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); align_combo.connect_changed(move |_| ts());
        let ts = trigger_save.clone(); update_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); check_monitor_label.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); check_cpu_metric.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); temp_unit_combo.connect_changed(move |_| ts());
        let ts = trigger_save.clone(); speed_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); realism_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); metrics_bright_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); matrix_bright_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); opac_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); check_occlusion.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); check_border.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); check_logging.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); check_ollama.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); commit_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); repos_entry.connect_changed(move |_| ts());
        let ts = trigger_save.clone(); check_weather_enabled.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); check_auto_loc.connect_toggled(move |_| ts());
        let ts = trigger_save.clone(); lat_spin.connect_value_changed(move |_| ts());
        let ts = trigger_save.clone(); lon_spin.connect_value_changed(move |_| ts());

        let ts_save = trigger_save.clone();
        btn_save.connect_clicked(move |_| {
            log::info!("Save & Apply clicked, triggering final save...");
            ts_save();
        });
        let tx_p = self.event_tx.clone();
        btn_purge.connect_clicked(move |_| { let _ = tx_p.send(GuiEvent::PurgeLogs); });
        let fs_p = font_spin.clone(); let rs_p = realism_spin.clone(); let ss_p = speed_spin.clone(); let us_p = update_spin.clone();
        btn_min.connect_clicked(move |_| { fs_p.set_value(14.0); rs_p.set_value(5.0); ss_p.set_value(0.5); us_p.set_value(2000.0); });
        let fs_m = font_spin.clone(); let rs_m = realism_spin.clone(); let ss_m = speed_spin.clone(); let us_m = update_spin.clone();
        btn_med.connect_clicked(move |_| { fs_m.set_value(16.0); rs_m.set_value(15.0); ss_m.set_value(1.0); us_m.set_value(1000.0); });
        let fs_e = font_spin.clone(); let rs_e = realism_spin.clone(); let ss_e = speed_spin.clone(); let us_e = update_spin.clone();
        btn_ext.connect_clicked(move |_| { fs_e.set_value(24.0); rs_e.set_value(50.0); ss_e.set_value(5.0); us_e.set_value(500.0); });

        let win_cancel = window.clone();
        btn_cancel.connect_clicked(move |_| { win_cancel.close(); });

        window.add(&main_vbox);
        window.show_all();
    }
}
