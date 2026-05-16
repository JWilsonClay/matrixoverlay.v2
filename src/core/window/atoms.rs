//! X11 Atom management for Sovereign Windowing.
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
    /// [HARDENED] Initializes X11 atoms with verified connection.
    pub fn new(conn: &xcb::Connection) -> Result<Self> {
        Ok(Self {
            wm_protocols: get_atom(conn, "WM_PROTOCOLS")?,
            wm_delete_window: get_atom(conn, "WM_DELETE_WINDOW")?,
            net_wm_window_type: get_atom(conn, "_NET_WM_WINDOW_TYPE")?,
            net_wm_window_type_dock: get_atom(conn, "_NET_WM_WINDOW_TYPE_DOCK")?,
            net_wm_state: get_atom(conn, "_NET_WM_STATE")?,
            net_wm_state_above: get_atom(conn, "_NET_WM_STATE_ABOVE")?,
            net_wm_state_sticky: get_atom(conn, "_NET_WM_STATE_STICKY")?,
            net_wm_desktop: get_atom(conn, "_NET_WM_DESKTOP")?,
            net_wm_strut: get_atom(conn, "_NET_WM_STRUT")?,
            net_wm_strut_partial: get_atom(conn, "_NET_WM_STRUT_PARTIAL")?,
        })
    }
}

fn get_atom(conn: &xcb::Connection, name: &str) -> Result<x::Atom> {
    let cookie = conn.send_request(&x::InternAtom { only_if_exists: false, name: name.as_bytes() });
    let reply = conn.wait_for_reply(cookie).context(format!("Atom fail: {}", name))?;
    Ok(reply.atom())
}
