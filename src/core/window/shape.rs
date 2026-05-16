//! X11 Shape Extension substrate for input passthrough.
//! Configures window regions to ensure non-blocking user interaction.

use xcb::{x, shape};
use crate::core::config::Config;

/// [HARDENED] Applies an empty input mask to make the window click-through.
/// This ensures the overlay does not steal focus or intercept user interactions.
pub fn apply_input_mask(conn: &xcb::Connection, window: x::Window, _w: u16, _h: u16, _config: &Config) {
    // [HARDENING] Set empty input region to guarantee click-through transparency
    conn.send_request(&shape::Rectangles {
        operation: shape::So::Set,
        destination_kind: shape::Sk::Input,
        ordering: x::ClipOrdering::Unsorted,
        destination_window: window,
        x_offset: 0,
        y_offset: 0,
        rectangles: &[],
    });
}
