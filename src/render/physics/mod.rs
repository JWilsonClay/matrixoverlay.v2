// src/render/physics/mod.rs
pub mod rain_stream;
pub mod rain_manager;

pub use self::rain_manager::RainManager;
pub use self::rain_stream::RainStream;
pub use self::rain_manager::{count_show_layout, take_survived};
