//! Sovereign Window Creation Substrate.
//! Handles secure X11 window allocation and property initialization.

use xcb::x;
use anyhow::{Context, Result, bail};
use crate::core::config::Config;
use crate::core::window::atoms::Atoms;

/// [HARDENED] Creates a secure X11 overlay window with verified dimensions.
pub fn create_window(
    conn: &xcb::Connection,
    screen_num: i32,
    atoms: &Atoms,
    _config: &Config,
    width: u16,
    height: u16,
    x: i16,
    y: i16,
) -> Result<x::Window> {
    if width == 0 || height == 0 {
        bail!("Invalid window dimensions: {}x{}", width, height);
    }
    if screen_num < 0 {
        bail!("Invalid screen index: {}", screen_num);
    }

    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize)
        .context("Screen not found")?;

    let depth = screen.allowed_depths()
        .find(|d| d.depth() == 32)
        .context("No 32-bit depth found")?;

    let visual = depth.visuals().first()
        .context("No 32-bit visual found")?;

    let window: x::Window = conn.generate_id();
    let colormap: x::Colormap = conn.generate_id();

    conn.send_request(&x::CreateColormap {
        alloc: x::ColormapAlloc::None,
        mid: colormap,
        window: screen.root(),
        visual: visual.visual_id(),
    });

    let values = [
        x::Cw::BackPixel(0),
        x::Cw::BorderPixel(0),
        x::Cw::OverrideRedirect(false),
        x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS | x::EventMask::STRUCTURE_NOTIFY),
        x::Cw::Colormap(colormap),
    ];

    log::debug!("[Window] Creating {}x{} window at ({}, {})", width, height, x, y);

    conn.send_request(&x::CreateWindow {
        depth: 32,
        wid: window,
        parent: screen.root(),
        x,
        y,
        width,
        height,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: visual.visual_id(),
        value_list: &values,
    });

    // Set EWMH properties (still useful as hints)
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: atoms.net_wm_window_type,
        r#type: x::ATOM_ATOM,
        data: &[atoms.net_wm_window_type_desktop],
    });

    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: atoms.net_wm_state,
        r#type: x::ATOM_ATOM,
        data: &[
            atoms.net_wm_state_below,
            atoms.net_wm_state_skip_taskbar,
            atoms.net_wm_state_skip_pager,
        ],
    });

    crate::core::window::shape::apply_click_through(conn, window)?;

    conn.send_request(&x::ConfigureWindow {
        window,
        value_list: &[x::ConfigWindow::StackMode(x::StackMode::Below)],
    });

    // Verify
    let geom_cookie = conn.send_request(&x::GetGeometry { drawable: x::Drawable::Window(window) });
    if let Ok(geom) = conn.wait_for_reply(geom_cookie) {
        if geom.width() != width || geom.height() != height {
            log::error!("[Window] WARNING: Created window has wrong size! Requested {}x{}, got {}x{}", width, height, geom.width(), geom.height());
        } else {
            log::debug!("[Window] Verified: Window {}x{} created successfully", geom.width(), geom.height());
        }
    }

    Ok(window)
}
