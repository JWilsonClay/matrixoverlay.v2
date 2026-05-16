// src/render/layout/mod.rs
pub mod formatting;
pub mod drawing;
pub mod components;

pub use self::formatting::{format_metric_value, format_bytes, parse_hex_color};
pub use self::drawing::{draw_occlusion_box, draw_text_glow_at};
pub use self::components::{draw_day_of_week, draw_metric_pair};
