pub mod config;
pub mod layout;
#[path = "metrics.rs"]
pub mod metrics_old;
#[path = "metrics/mod.rs"]
pub mod metrics;
#[path = "render.rs"]
pub mod render_old;
#[path = "render/mod.rs"]
pub mod render;
pub mod tray;
pub mod window;
pub mod timer;
pub mod path_utils;
pub mod logging;
pub mod version;
pub mod build_logger;
pub mod gui;

// --- REFACTOR BRIDGES (Phase 1) ---
pub mod core;
pub mod ui;