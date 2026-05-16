// src/ui/gui/mod.rs
pub mod general;
pub mod cosmetics;
pub mod productivity;
pub mod weather;
pub mod metrics;
pub mod advanced;

use gtk::prelude::*;
use gtk::{Window, WindowType, Notebook, Box, Orientation, Label, Button};
use std::sync::Arc;
use crossbeam_channel::Sender;
use crate::core::config::Config;
use crate::core::logging;

pub enum GuiEvent {
    Reload,
    PurgeLogs,
    OpenConfig,
    UpdateAvailable(String),
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
        
        let vbox_gen = Box::new(Orientation::Vertical, 10);
        let gen_w = general::build(&vbox_gen, &self.config);
        notebook.append_page(&vbox_gen, Some(&Label::new(Some("General"))));

        let vbox_cos = Box::new(Orientation::Vertical, 10);
        let cos_w = cosmetics::build(&vbox_cos, &self.config);
        notebook.append_page(&vbox_cos, Some(&Label::new(Some("Cosmetics"))));

        let vbox_prod = Box::new(Orientation::Vertical, 10);
        let prod_w = productivity::build(&vbox_prod, &self.config);
        notebook.append_page(&vbox_prod, Some(&Label::new(Some("Productivity"))));

        let vbox_weath = Box::new(Orientation::Vertical, 10);
        let weath_w = weather::build(&vbox_weath, &self.config);
        notebook.append_page(&vbox_weath, Some(&Label::new(Some("Weather"))));

        let vbox_adv = Box::new(Orientation::Vertical, 10);
        let adv_w = advanced::build(&vbox_adv, &self.config);
        notebook.append_page(&vbox_adv, Some(&Label::new(Some("Advanced"))));

        let main_vbox = Box::new(Orientation::Vertical, 10);
        main_vbox.pack_start(&notebook, true, true, 5);
        
        let hbox_btns = Box::new(Orientation::Horizontal, 10);
        let btn_cancel = Button::with_label("Cancel");
        let btn_save = Button::with_label("Save & Apply Changes");
        hbox_btns.pack_end(&btn_save, false, false, 5);
        hbox_btns.pack_end(&btn_cancel, false, false, 5);
        main_vbox.pack_start(&hbox_btns, false, false, 10);

        let tx = self.event_tx.clone();
        let config_save = self.config.clone();
        
        btn_save.connect_clicked(move |_| {
            let mut new_config = (*config_save).clone();
            
            // Priority 1 Fix: Extract values from widgets using correct GTK 0.16 methods
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

            // Cosmetics
            new_config.cosmetics.rain_speed = cos_w.0.value();
            new_config.cosmetics.realism_scale = cos_w.1.value() as u32;
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

            // Save and Reload
            if let Err(e) = new_config.save() {
                log::error!("Failed to save config: {}", e);
            } else {
                let _ = tx.send(GuiEvent::Reload);
            }
        });

        let tx_p = self.event_tx.clone();
        adv_w.4.connect_clicked(move |_| { let _ = tx_p.send(GuiEvent::PurgeLogs); });

        let win_cancel = window.clone();
        btn_cancel.connect_clicked(move |_| { win_cancel.close(); });

        window.add(&main_vbox);
        window.show_all();
    }
}
