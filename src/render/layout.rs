//! Matrix Overlay Layout Substrate.
pub mod layout;

pub use self::layout::formatting::{format_metric_value, format_bytes, parse_hex_color};
pub use self::layout::drawing::{draw_occlusion_box, draw_text_glow_at};
pub use self::layout::components::{draw_day_of_week, draw_metric_pair};
