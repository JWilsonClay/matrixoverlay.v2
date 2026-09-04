//! Advanced Configuration GUI Substrate.
//! Orchestrates performance presets and maintenance interactions.

use gtk::prelude::*;
use gtk::{Box, Orientation, Label, CheckButton, Button};
use crate::core::config::Config;

/// [HARDENED] Builds the advanced configuration view with failure-isolated widgets.
pub fn build(vbox: &Box, config: &Config) -> (Button, Button, Button, CheckButton, Button) {
    vbox.set_border_width(10);
    
    let btn_min = Button::with_label("Minimal (needs Pulse Mode — not yet available)");
    let btn_med = Button::with_label("Medium");
    let btn_ext = Button::with_label("Extreme (exceeds the ambience budget)");
    
    let hbox_presets = Box::new(Orientation::Horizontal, 5);
    hbox_presets.pack_start(&btn_min, true, true, 0);
    hbox_presets.pack_start(&btn_med, true, true, 0);
    hbox_presets.pack_start(&btn_ext, true, true, 0);
    
    // [Phase 8.5] Reflect the active preset rather than leaving the user to guess.
    vbox.pack_start(&Label::new(Some("Performance Presets")), false, false, 0);
    vbox.pack_start(
        &Label::new(Some(&format!("Active: {}", config.cosmetics.perf_preset) as &str)),
        false, false, 0,
    );
    vbox.pack_start(&hbox_presets, false, false, 0);

    let check_logging = CheckButton::with_label("Enable Debug Logging (Auto-rotated)");
    check_logging.set_active(config.logging.enabled);
    vbox.pack_start(&check_logging, false, false, 10);

    let btn_purge = Button::with_label("Purge Debug Logs (/tmp)");
    vbox.pack_start(&Label::new(Some("Maintenance")), false, false, 5);
    vbox.pack_start(&btn_purge, false, false, 0);

    (btn_min, btn_med, btn_ext, check_logging, btn_purge)
}
