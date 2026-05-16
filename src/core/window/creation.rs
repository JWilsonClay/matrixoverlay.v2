// src/core/window/creation.rs
use xcb::x;
use anyhow::{Context, Result};
use crate::core::config::Config;
use crate::core::window::atoms::Atoms;

pub fn create_window(
    conn: &xcb::Connection,
    screen_num: i32,
    atoms: &Atoms,
    config: &Config,
    width: u16,
    height: u16,
    x: i16,
    y: i16,
) -> Result<x::Window> {
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).context("No screen found")?;
    
    let window: x::Window = conn.generate_id();

    let values = [
        x::Cw::BackPixel(screen.black_pixel()),
        x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS | x::EventMask::STRUCTURE_NOTIFY),
        x::Cw::OverrideRedirect(true),
    ];

    conn.send_request(&x::CreateWindow {
        depth: x::COPY_FROM_PARENT as u8,
        wid: window,
        parent: screen.root(),
        x,
        y,
        width,
        height,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: screen.root_visual(),
        value_list: &values,
    });

    // Set Window Types
    let dock_type = [atoms.net_wm_window_type_dock];
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window,
        property: atoms.net_wm_window_type,
        r#type: x::ATOM_ATOM,
        data: &dock_type,
    });

    Ok(window)
}
