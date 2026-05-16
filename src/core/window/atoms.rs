//! X11 Atom management for Sovereign Windowing.
//! Provides cached atom identifiers for EWMH and WM communication.

use xcb::x;
use anyhow::{Context, Result};

/// [HARDENED] Cached X11 atoms for protocol compliance.
pub struct Atoms {
    pub wm_protocols: x::Atom,
    pub wm_delete_window: x::Atom,
    pub net_wm_window_type: x::Atom,
    pub net_wm_window_type_dock: x::Atom,
    pub net_wm_window_type_desktop: x::Atom,
    pub net_wm_state: x::Atom,
    pub net_wm_state_above: x::Atom,
    pub net_wm_state_below: x::Atom,
    pub net_wm_state_sticky: x::Atom,
    pub net_wm_state_skip_taskbar: x::Atom,
    pub net_wm_state_skip_pager: x::Atom,
    pub net_wm_desktop: x::Atom,
    pub net_wm_strut: x::Atom,
    pub net_wm_strut_partial: x::Atom,
}

impl Atoms {
    /// [HARDENED] Initializes X11 atoms with verified connection.
    pub fn new(conn: &xcb::Connection) -> Result<Self> {
        Ok(Self {
            wm_protocols: intern_atom(conn, "WM_PROTOCOLS")?,
            wm_delete_window: intern_atom(conn, "WM_DELETE_WINDOW")?,
            net_wm_window_type: intern_atom(conn, "_NET_WM_WINDOW_TYPE")?,
            net_wm_window_type_dock: intern_atom(conn, "_NET_WM_WINDOW_TYPE_DOCK")?,
            net_wm_window_type_desktop: intern_atom(conn, "_NET_WM_WINDOW_TYPE_DESKTOP")?,
            net_wm_state: intern_atom(conn, "_NET_WM_STATE")?,
            net_wm_state_above: intern_atom(conn, "_NET_WM_STATE_ABOVE")?,
            net_wm_state_below: intern_atom(conn, "_NET_WM_STATE_BELOW")?,
            net_wm_state_sticky: intern_atom(conn, "_NET_WM_STATE_STICKY")?,
            net_wm_state_skip_taskbar: intern_atom(conn, "_NET_WM_STATE_SKIP_TASKBAR")?,
            net_wm_state_skip_pager: intern_atom(conn, "_NET_WM_STATE_SKIP_PAGER")?,
            net_wm_desktop: intern_atom(conn, "_NET_WM_DESKTOP")?,
            net_wm_strut: intern_atom(conn, "_NET_WM_STRUT")?,
            net_wm_strut_partial: intern_atom(conn, "_NET_WM_STRUT_PARTIAL")?,
        })
    }
}

/// [HARDENED] Deterministic atom interning.
fn intern_atom(conn: &xcb::Connection, name: &str) -> Result<x::Atom> {
    let cookie = conn.send_request(&x::InternAtom { only_if_exists: false, name: name.as_bytes() });
    let reply = conn.wait_for_reply(cookie).context(format!("Atom fail: {}", name))?;
    Ok(reply.atom())
}
