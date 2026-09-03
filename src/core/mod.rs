//! Core Substrate for Matrix Overlay.
//! [HARDENED] All modules are independently testable and strictly bounded.

pub mod main;
pub mod config;
pub mod layout;
pub mod productivity;
pub mod window;
pub mod logging;
pub mod init;
pub mod threads;
pub mod path_utils;
pub mod timer;
pub mod telemetry;
pub mod update;
pub mod version;
