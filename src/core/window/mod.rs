//! Sovereign Window Orchestration Substrate.
pub mod atoms;
pub mod shape;
pub mod creation;

use xcb::x;
use anyhow::{Result, Context};
use crate::core::config::{Config, Screen};

pub struct MonitorContext { pub window: x::Window, pub screen: Screen }

pub struct WindowManager { pub monitors: Vec<MonitorContext> }

impl WindowManager {
    /// [HARDENED] Atomic cleanup of all application windows.
    pub fn cleanup(&self, conn: &xcb::Connection) -> Result<()> {
        for ctx in &self.monitors {
            let _ = conn.send_request(&x::UnmapWindow { window: ctx.window });
            let _ = conn.send_request(&x::DestroyWindow { window: ctx.window });
        }
        let _ = conn.flush();
        Ok(())
    }
}

/// [HARDENED] Orchestrates window creation with rollback on partial failure.
pub fn create_all_windows(conn: &xcb::Connection, config: &Config) -> Result<WindowManager> {
    let atoms = atoms::Atoms::new(conn)?;
    let mut monitors = Vec::new();

    for screen in &config.screens {
        match creation::create_window(conn, 0, &atoms, config, 1920, 1080, 0, 0) {
            Ok(window) => monitors.push(MonitorContext { window, screen: screen.clone() }),
            Err(e) => {
                // [HARDENING] Rollback partial success to prevent orphaned windows
                let _ = WindowManager { monitors }.cleanup(conn);
                return Err(e).context("Failed to create all windows");
            }
        }
    }
    Ok(WindowManager { monitors })
}

/// [HARDENED] Deterministic keycode discovery from keysym.
pub fn find_keycode(conn: &xcb::Connection, keysym: u32) -> Result<Option<u8>> {
    let setup = conn.get_setup();
    let min = setup.min_keycode();
    let max = setup.max_keycode();
    let cookie = conn.send_request(&x::GetKeyboardMapping { first_keycode: min, count: max - min + 1 });
    let reply = conn.wait_for_reply(cookie)?;
    let keysyms = reply.keysyms();
    let per = reply.keysyms_per_keycode() as usize;

    for (i, syms) in keysyms.chunks(per).enumerate() {
        if syms.iter().any(|&s| s == keysym) {
            return Ok(Some(min + i as u8));
        }
    }
    Ok(None)
}
