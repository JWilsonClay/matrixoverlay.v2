//! Weather Configuration GUI Substrate.
//! Orchestrates location parameters and automated Geo-IP discovery.

use gtk::prelude::*;
use gtk::{Box, Label, SpinButton, CheckButton};
use crate::core::config::Config;

/// [HARDENED] Builds the weather configuration view with strictly bounded inputs.
pub fn build(vbox: &Box, config: &Config) -> (CheckButton, CheckButton, SpinButton, SpinButton) {
    vbox.set_border_width(10);

    let check_weather_enabled = CheckButton::with_label("Enable Weather Data (Open-Meteo)");
    check_weather_enabled.set_active(config.weather.enabled);
    vbox.pack_start(&check_weather_enabled, false, false, 0);

    let check_auto_loc = CheckButton::with_label("Automatic Location (Geo-IP)");
    check_auto_loc.set_active(config.weather.auto_location);
    vbox.pack_start(&check_auto_loc, false, false, 0);

    vbox.pack_start(&Label::new(Some("Manual Latitude")), false, false, 5);
    let lat_spin = SpinButton::with_range(-90.0, 90.0, 0.0001);
    lat_spin.set_value(config.weather.lat);
    vbox.pack_start(&lat_spin, false, false, 0);

    vbox.pack_start(&Label::new(Some("Manual Longitude")), false, false, 5);
    let lon_spin = SpinButton::with_range(-180.0, 180.0, 0.0001);
    lon_spin.set_value(config.weather.lon);
    vbox.pack_start(&lon_spin, false, false, 0);

    // [HARDENING] Interactive sensitivity logic
    let cal = check_auto_loc.clone();
    let ls = lat_spin.clone();
    let lns = lon_spin.clone();
    let update_ws = move |enabled: bool| {
        cal.set_sensitive(enabled);
        let auto = cal.is_active();
        ls.set_sensitive(enabled && !auto);
        lns.set_sensitive(enabled && !auto);
    };
    update_ws(config.weather.enabled);
    
    let uws_c = update_ws;
    check_weather_enabled.connect_toggled(move |cb| uws_c(cb.is_active()));
    
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

            // [SECURITY] Failure-isolated Geo-IP discovery
            std::thread::spawn(move || {
                if let Ok((lat, lon)) = crate::metrics::fetch_geoip_location() {
                     let _ = tx.send((lat, lon));
                }
            });
        }
    });

    (check_weather_enabled, check_auto_loc, lat_spin, lon_spin)
}
