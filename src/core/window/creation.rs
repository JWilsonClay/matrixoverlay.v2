//! Sovereign Window Creation Substrate.
//! Handles secure X11 window allocation and property initialization.

use xcb::x;
use anyhow::{Context, Result, bail};
use crate::core::config::Config;
use crate::core::window::atoms::Atoms;

/// [HARDENED] Creates a secure X11 overlay window with verified dimensions and screen index.
pub fn create_window(
    conn: &xcb::Connection, screen_num: i32, atoms: &Atoms, _config: &Config,
    width: u16, height: u16, x: i16, y: i16,
) -> Result<x::Window> {
    // [HARDENING] Strict geometry and index validation
    if width == 0 || height == 0 { bail!("Invalid window dimensions: {}x{}", width, height); }
    if screen_num < 0 { bail!("Invalid screen index: {}", screen_num); }

    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).context("Screen not found")?;
    let window: x::Window = conn.generate_id();

    // [SECURITY] OverrideRedirect(true) bypasses WM for total overlay control.
    // [SECURITY] EventMask restricts interaction to minimum necessary substrate.
    let values = [
        x::Cw::BackPixel(screen.black_pixel()),
        x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS | x::EventMask::STRUCTURE_NOTIFY),
        x::Cw::OverrideRedirect(true),
    ];

    conn.send_request(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8, wid: window, parent: screen.root(),
        x, y, width, height, border_width: 0,
        class: x::WindowClass::InputOutput, visual: screen.root_visual(),
        value_list: &values,
    });

    // [HARDENING] Establish EWMH compliance for dock-type behavior
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace, window, property: atoms.net_wm_window_type,
        r#type: x::ATOM_ATOM, data: &[atoms.net_wm_window_type_dock],
    });

    Ok(window)
}
