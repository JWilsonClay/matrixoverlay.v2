// src/core/window/shape.rs
use xcb::{x, shape};
use crate::core::config::Config;

pub fn apply_input_mask(conn: &xcb::Connection, window: x::Window, width: u16, height: u16, _config: &Config) {
    // Logic to make window click-through while allowing drawing.
    // We use an empty rectangle for the Input shape.
    let rects = [];
    let _ = conn.send_request(&shape::Rectangles {
        operation: shape::So::Set,
        destination_kind: shape::Sk::Input,
        ordering: x::ClipOrdering::Unsorted,
        destination_window: window,
        x_offset: 0,
        y_offset: 0,
        rectangles: &rects,
    });
}
