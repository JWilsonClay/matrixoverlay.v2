// src/core/window/atoms.rs
use xcb::x;
use anyhow::{Context, Result};

pub struct Atoms {
    pub wm_protocols: x::Atom,
    pub wm_delete_window: x::Atom,
    pub net_wm_window_type: x::Atom,
    pub net_wm_window_type_dock: x::Atom,
    pub net_wm_state: x::Atom,
    pub net_wm_state_above: x::Atom,
    pub net_wm_state_sticky: x::Atom,
    pub net_wm_desktop: x::Atom,
    pub net_wm_strut: x::Atom,
    pub net_wm_strut_partial: x::Atom,
}

impl Atoms {
    pub fn new(conn: &xcb::Connection) -> Result<Self> {
        let wm_protocols = get_atom(conn, "WM_PROTOCOLS")?;
        let wm_delete_window = get_atom(conn, "WM_DELETE_WINDOW")?;
        let net_wm_window_type = get_atom(conn, "_NET_WM_WINDOW_TYPE")?;
        let net_wm_window_type_dock = get_atom(conn, "_NET_WM_WINDOW_TYPE_DOCK")?;
        let net_wm_state = get_atom(conn, "_NET_WM_STATE")?;
        let net_wm_state_above = get_atom(conn, "_NET_WM_STATE_ABOVE")?;
        let net_wm_state_sticky = get_atom(conn, "_NET_WM_STATE_STICKY")?;
        let net_wm_desktop = get_atom(conn, "_NET_WM_DESKTOP")?;
        let net_wm_strut = get_atom(conn, "_NET_WM_STRUT")?;
        let net_wm_strut_partial = get_atom(conn, "_NET_WM_STRUT_PARTIAL")?;

        Ok(Self {
            wm_protocols,
            wm_delete_window,
            net_wm_window_type,
            net_wm_window_type_dock,
            net_wm_state,
            net_wm_state_above,
            net_wm_state_sticky,
            net_wm_desktop,
            net_wm_strut,
            net_wm_strut_partial,
        })
    }
}

fn get_atom(conn: &xcb::Connection, name: &str) -> Result<x::Atom> {
    let cookie = conn.send_request(&x::InternAtom {
        only_if_exists: false,
        name: name.as_bytes(),
    });
    let reply = conn.wait_for_reply(cookie).context(format!("Failed to intern atom {}", name))?;
    Ok(reply.atom())
}
