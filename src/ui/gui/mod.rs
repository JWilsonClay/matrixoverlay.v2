//! Sovereign GUI Orchestration Substrate.
pub mod general;
pub mod cosmetics;
pub mod productivity;
pub mod weather;
pub mod metrics;
pub mod advanced;
pub mod logic;

use gtk::prelude::*;
use gtk::{Window, WindowType, Notebook, Box, Orientation, Label, Button};
use std::sync::Arc;
use crossbeam_channel::Sender;
use crate::core::config::Config;

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
        Self { config: Arc::new(config), event_tx }
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
        let cfg_arc = self.config.clone();
        
        btn_save.connect_clicked(move |_| {
            let mut new_cfg = (*cfg_arc).clone();
            logic::update_config_from_widgets(&mut new_cfg, &gen_w, &cos_w, &weath_w);
            if let Err(e) = new_cfg.save() {
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
