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
    update_manager: Arc<crate::core::update::UpdateManager>,
) {
    thread::spawn(move || {
        let mut current_config = config.clone();
        let wm = match create_all_windows(&conn, &current_config) {
            Ok(m) => m,
            Err(e) => { log::error!("Failed to create windows: {}", e); return; }
        };

        let mut renderers = Vec::new();
        for (i, ctx) in wm.monitors.iter().enumerate() {
            // [HARDENING] Retrieve physical geometry from the monitor context
            let cookie = conn.send_request(&xcb::x::GetGeometry { drawable: xcb::x::Drawable::Window(ctx.window) });
            if let Ok(geom) = conn.wait_for_reply(cookie) {
                let w = geom.width();
                let h = geom.height();
                let layout = crate::core::layout::compute(&current_config, i, w, h);
                if let Ok(r) = Renderer::new(&conn, w, h, i, layout, &current_config) {
                    renderers.push(r);
                }
            }
        }
        
        let (tick_tx, tick_rx) = bounded(1);
        spawn_tick_thread(tick_tx, Arc::clone(&shutdown));

        let key_w = find_keycode(&conn, 0x0077).unwrap_or(Some(0)).unwrap_or(0);
        let key_q = find_keycode(&conn, 0x0071).unwrap_or(Some(0)).unwrap_or(0);
        let (mut visible, mut last_draw) = (true, Instant::now());
        
        // [HARDENING] Ensure windows are mapped on startup and pushed to the absolute bottom
        for ctx in &wm.monitors {
            let _ = conn.send_request(&xcb::x::MapWindow { window: ctx.window });
            let _ = conn.send_request(&xcb::x::ConfigureWindow {
                window: ctx.window,
                value_list: &[xcb::x::ConfigWindow::StackMode(xcb::x::StackMode::Below)],
            });
        }
        let _ = conn.flush();
        
        let mut latest_version: Option<String> = None;

        loop {
            if shutdown.load(Ordering::SeqCst) { break; }
            select! {
                recv(xcb_rx) -> res => if let Ok(ev) = res { handle_xcb_event(ev, &conn, &wm, &mut visible, &metrics, &mut renderers, &current_config, &mut last_draw, key_w, key_q, &shutdown); },
                recv(tick_rx) -> _ => if visible { draw_frame(&conn, &wm, &mut renderers, &current_config, &metrics, &mut last_draw); },
                recv(MenuEvent::receiver()) -> res => if let Ok(ev) = res { handle_menu_event(&conn, ev, &mut current_config, &mut renderers, &metrics_tx, &control_tx, &shutdown, &update_manager, &mut latest_version); },
                recv(gui_rx) -> res => if let Ok(ev) = res { handle_gui_event(&conn, ev, &mut current_config, &mut renderers, &metrics_tx); },
                recv(update_rx) -> res => if let Ok(ev) = res { handle_update_event(ev, &control_tx, &mut latest_version); }
            }
        }
        let _ = wm.cleanup(&conn);
    });
}

fn spawn_tick_thread(tx: Sender<()>, shutdown: Arc<AtomicBool>) {
    thread::spawn(move || {
        let interval = Duration::from_millis(33); 
        while !shutdown.load(Ordering::SeqCst) {
            let start = Instant::now();
            if tx.send(()).is_err() { break; }
            let elapsed = start.elapsed();
            if elapsed < interval { thread::sleep(interval - elapsed); }
            else { thread::sleep(Duration::from_millis(1)); }
        }
    });
}
