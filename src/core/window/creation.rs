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
    
    // [HARDENING] Find 32-bit visual for transparency
    let depth = screen.allowed_depths().find(|d| d.depth() == 32).context("32-bit depth not found")?;
    let visual = depth.visuals().first().context("No 32-bit visual found")?;
    
    let window: x::Window = conn.generate_id();
    let colormap: x::Colormap = conn.generate_id();
    
    conn.send_request(&x::CreateColormap {
        alloc: x::ColormapAlloc::None, mid: colormap, window: screen.root(), visual: visual.visual_id(),
    });

    let values = [
        x::Cw::BackPixel(0),
        x::Cw::BorderPixel(0),
        x::Cw::OverrideRedirect(true),
        x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS | x::EventMask::STRUCTURE_NOTIFY),
        x::Cw::Colormap(colormap),
    ];

    conn.send_request(&x::CreateWindow {
        depth: 32, wid: window, parent: screen.root(),
        x, y, width, height, border_width: 0,
        class: x::WindowClass::InputOutput, visual: visual.visual_id(),
        value_list: &values,
    });

    // [HARDENING] Establish EWMH compliance for DESKTOP type
    // Using net_wm_window_type_desktop for maximum "below-everything" stability
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace, window, property: atoms.net_wm_window_type,
        r#type: x::ATOM_ATOM, data: &[atoms.net_wm_window_type_desktop],
    });

    // [HARDENING] Set _NET_WM_STATE to skip taskbar/pager and stay below
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace, window, property: atoms.net_wm_state,
        r#type: x::ATOM_ATOM, data: &[atoms.net_wm_state_below, atoms.net_wm_state_skip_taskbar, atoms.net_wm_state_skip_pager],
    });

    // Apply click-through shape
    crate::core::window::shape::apply_click_through(conn, window)?;

    Ok(window)
}
