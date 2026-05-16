// src/core/threads/mod.rs
pub mod handlers;

use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use crossbeam_channel::{Sender, Receiver, select, bounded};
use tray_icon::menu::MenuEvent;

use crate::core::config::Config;
use crate::core::window::{create_all_windows, find_keycode};
use crate::core::{layout, logging, productivity};
use crate::metrics::{MetricsCommand, SharedMetrics};
use crate::render::Renderer;
use crate::ui::gui::GuiEvent;
use crate::ui::tray::SystemTray;
use crate::core::update::UpdateEvent;

pub use self::handlers::*;

pub fn spawn_xcb_thread(conn: Arc<xcb::Connection>, xcb_tx: Sender<xcb::Event>) {
    thread::spawn(move || {
        let conn_event = Arc::clone(&conn);
        loop {
            match conn_event.wait_for_event() {
                Ok(event) => { if xcb_tx.send(event).is_err() { break; } }
                Err(e) => { log::error!("XCB Connection Error: {}", e); break; }
            }
        }
    });
}

pub fn spawn_productivity_thread(config: Config, shutdown: Arc<AtomicBool>) {
    thread::spawn(move || {
        let mut last_check = Instant::now();
        while !shutdown.load(Ordering::Relaxed) {
            if last_check.elapsed() >= Duration::from_secs(3600) {
                last_check = Instant::now();
                let _ = productivity::run_auto_commit_cycle(&config);
            }
            thread::sleep(Duration::from_secs(60));
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_overlay_thread(
    conn: Arc<xcb::Connection>,
    config: Config,
    shutdown: Arc<AtomicBool>,
    metrics: Arc<std::sync::Mutex<SharedMetrics>>,
    metrics_tx: Sender<MetricsCommand>,
    xcb_rx: Receiver<xcb::Event>,
    gui_rx: Receiver<GuiEvent>,
    control_tx: Sender<GuiEvent>,
    update_rx: Receiver<UpdateEvent>,
) {
    thread::spawn(move || {
        let mut current_config = config.clone();
        let wm = match create_all_windows(&conn, &current_config) {
            Ok(m) => m,
            Err(e) => { log::error!("Failed to create windows: {}", e); return; }
        };

        let mut renderers = Vec::new();
        for (i, _) in wm.monitors.iter().enumerate() {
            let layout = layout::compute(&current_config, i, 1920, 1080);
            if let Ok(renderer) = Renderer::new(1920, 1080, i, layout, &current_config) {
                renderers.push(renderer);
            }
        }
        
        let (tick_tx, tick_rx) = bounded(1);
        spawn_tick_thread(tick_tx);

        let key_w = find_keycode(&conn, 0x0077).unwrap_or(Some(0)).unwrap_or(0);
        let key_q = find_keycode(&conn, 0x0071).unwrap_or(Some(0)).unwrap_or(0);
        let mut visible = true;
        let mut last_draw = Instant::now();

        loop {
            if shutdown.load(Ordering::Relaxed) { break; }
            select! {
                recv(xcb_rx) -> res => if let Ok(ev) = res { handle_xcb_event(ev, &conn, &wm, &mut visible, &metrics, &mut renderers, &current_config, &mut last_draw, key_w, key_q, &shutdown); },
                recv(tick_rx) -> _ => if visible { draw_frame(&conn, &wm, &mut renderers, &current_config, &metrics, &mut last_draw); },
                recv(MenuEvent::receiver()) -> res => if let Ok(ev) = res { handle_menu_event(ev, &mut current_config, &mut renderers, &metrics_tx, &control_tx, &shutdown); },
                recv(gui_rx) -> res => if let Ok(ev) = res { handle_gui_event(ev, &mut current_config, &mut renderers, &metrics_tx); },
                recv(update_rx) -> res => if let Ok(ev) = res { handle_update_event(ev, &control_tx); }
            }
        }
        let _ = wm.cleanup(&conn);
    });
}

fn spawn_tick_thread(tx: Sender<()>) {
    thread::spawn(move || {
        let interval = Duration::from_millis(33); 
        loop {
            let start = Instant::now();
            if tx.send(()).is_err() { break; }
            let elapsed = start.elapsed();
            if elapsed < interval { thread::sleep(interval - elapsed); }
            else { thread::sleep(Duration::from_millis(1)); }
        }
    });
}
