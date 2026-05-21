//! Sovereign Window Orchestration Substrate.
//! Manages X11 window lifecycles, monitor mapping, and input discovery.

pub mod atoms;
pub mod shape;
pub mod creation;

use xcb::x;
use xcb::Xid;
use anyhow::{Result, Context};
use crate::core::config::{Config, Screen};

/// Context for a single monitor window.
pub struct MonitorContext {
    pub window: x::Window,
    pub screen: Screen,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

/// [HARDENED] Orchestrates multiple windows with failure isolation.
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

pub struct MonitorInfo {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub name: String,
}

pub fn create_all_windows(conn: &xcb::Connection, config: &Config) -> Result<WindowManager> {
    let atoms = atoms::Atoms::new(conn)?;
    let mut monitors = Vec::new();

    let setup = conn.get_setup();
    let screen = setup.roots().next().context("No X11 screen found")?;

    let randr_cookie = conn.send_request(&xcb::randr::GetScreenResources { window: screen.root() });
    let randr_reply = conn.wait_for_reply(randr_cookie)?;

    let mut physical_monitors = Vec::new();
    for &output in randr_reply.outputs() {
        let output_cookie = conn.send_request(&xcb::randr::GetOutputInfo {
            output,
            config_timestamp: randr_reply.config_timestamp(),
        });
        if let Ok(output_info) = conn.wait_for_reply(output_cookie) {
            if output_info.connection() == xcb::randr::Connection::Connected
                && output_info.crtc() != xcb::randr::Crtc::none()
            {
                let crtc_cookie = conn.send_request(&xcb::randr::GetCrtcInfo {
                    crtc: output_info.crtc(),
                    config_timestamp: randr_reply.config_timestamp(),
                });
                if let Ok(crtc_info) = conn.wait_for_reply(crtc_cookie) {
                    physical_monitors.push(MonitorInfo {
                        x: crtc_info.x(),
                        y: crtc_info.y(),
                        width: crtc_info.width(),
                        height: crtc_info.height(),
                        name: String::from_utf8_lossy(output_info.name()).to_string(),
                    });
                }
            }
        }
    }

    if physical_monitors.is_empty() {
        log::warn!("RandR discovery returned 0 monitors. Falling back.");
        physical_monitors.push(MonitorInfo { x: 0, y: 0, width: 1920, height: 1080, name: "Fallback".to_string() });
    }

    for (i, info) in physical_monitors.iter().enumerate() {
        log::info!(
            "[HUD] Deploying to monitor {}: {} ({}x{} at {},{})",
            i, info.name, info.width, info.height, info.x, info.y
        );

        let screen_cfg = config.screens.get(i).cloned().unwrap_or_else(|| config.screens[0].clone());

        match creation::create_window(conn, 0, &atoms, config, info.width, info.height, info.x, info.y) {
            Ok(window) => {
                // [FIX] More aggressive mapping with explicit flush
                conn.send_request(&x::MapWindow { window });
                conn.send_request(&x::ConfigureWindow {
                    window,
                    value_list: &[x::ConfigWindow::StackMode(x::StackMode::Below)],
                });
                
                // Force the requests to be sent to the X server immediately
                let _ = conn.flush();

                monitors.push(MonitorContext {
                    window,
                    screen: screen_cfg,
                    name: info.name.clone(),
                    x: info.x,
                    y: info.y,
                    width: info.width,
                    height: info.height,
                });
                
                log::debug!("[HUD] Window {} mapped and configured for monitor {}", i, info.name);
            }
            Err(e) => {
                log::error!("[HUD] Failed to create window for monitor {} ({}): {}", i, info.name, e);
                let _ = WindowManager { monitors }.cleanup(conn);
                return Err(e).context(format!("Failed to create window for monitor {}", i));
            }
        }
    }

    let _ = conn.flush();
    Ok(WindowManager { monitors })
}

pub fn find_keycode(conn: &xcb::Connection, keysym: u32) -> Result<Option<u8>> {
    let setup = conn.get_setup();
    let min = setup.min_keycode();
    let max = setup.max_keycode();
    let cookie = conn.send_request(&x::GetKeyboardMapping {
        first_keycode: min,
        count: max - min + 1,
    });
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
