// src/core/window/mod.rs
pub mod atoms;
pub mod shape;
pub mod creation;

use xcb::x;
use anyhow::Result;
use crate::core::config::{Config, Screen};

pub struct MonitorContext {
    pub window: x::Window,
    pub screen: Screen,
}

pub struct WindowManager {
    pub monitors: Vec<MonitorContext>,
}

impl WindowManager {
    pub fn cleanup(&self, conn: &xcb::Connection) -> Result<()> {
        for ctx in &self.monitors {
            let _ = conn.send_request(&x::UnmapWindow { window: ctx.window });
            let _ = conn.send_request(&x::DestroyWindow { window: ctx.window });
        }
        let _ = conn.flush();
        Ok(())
    }
}

pub fn create_all_windows(conn: &xcb::Connection, config: &Config) -> Result<WindowManager> {
    let atoms = atoms::Atoms::new(conn)?;
    let mut monitors = Vec::new();

    // Placeholder monitor discovery
    for screen in &config.screens {
         let window = creation::create_window(conn, 0, &atoms, config, 1920, 1080, 0, 0)?;
         monitors.push(MonitorContext {
             window,
             screen: screen.clone(),
         });
    }
    
    Ok(WindowManager { monitors })
}

pub fn find_keycode(_conn: &xcb::Connection, _keysym: u32) -> Result<Option<u8>> {
    Ok(Some(0))
}

pub fn grab_key_combinations(_conn: &xcb::Connection, _config: &Config) -> Result<()> {
    Ok(())
}
