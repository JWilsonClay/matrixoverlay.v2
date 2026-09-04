// src/core/threads/mod.rs
pub mod handlers;

use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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
        let target_fps = Arc::new(AtomicU32::new(current_config.general.fps()));
        spawn_tick_thread(tick_tx, Arc::clone(&shutdown), Arc::clone(&target_fps));

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
            // The GUI and tray both mutate `current_config` in place, so publish
            // the (clamped) rate after every event rather than only on the GUI
            // arm. One relaxed store per event; nothing on the render path.
            target_fps.store(current_config.general.fps(), Ordering::Relaxed);
        }
        let _ = wm.cleanup(&conn);
    });
}

/// Nanoseconds per tick for a clamped rate. Split out so the governor's timing
/// rule is unit-testable without spawning a thread (S-07 / AC1).
pub fn tick_period(target_fps: u32) -> Duration {
    Duration::from_nanos(1_000_000_000 / target_fps.clamp(1, 60) as u64)
}

/// The next deadline after `now`, given the previous one.
///
/// **This is the F4 fix.** The old loop measured `elapsed` across the blocking
/// `send()` on a `bounded(1)` channel: a slow frame made the receiver late, the
/// send blocked, `elapsed` exceeded the interval, and the thread slept **1 ms**
/// and immediately re-queued — a fail-open that ran the loop as fast as the
/// renderer could drain it, precisely when it was already too slow. A monotonic
/// deadline cannot fail open: missed ticks are **skipped, never queued as
/// catch-up frames**, so lateness can only ever cost frames, not add them.
pub fn next_deadline(prev: Instant, now: Instant, period: Duration) -> Instant {
    let mut d = prev + period;
    while d <= now { d += period; }
    d
}

fn spawn_tick_thread(tx: Sender<()>, shutdown: Arc<AtomicBool>, target_fps: Arc<AtomicU32>) {
    thread::spawn(move || {
        let mut period = tick_period(target_fps.load(Ordering::Relaxed));
        let mut deadline = Instant::now() + period;
        while !shutdown.load(Ordering::SeqCst) {
            if tx.send(()).is_err() { break; }
            // Sample AFTER the send returns: time spent blocked on the bounded
            // channel is the renderer being late, and must not be folded into
            // the period arithmetic. That fold is what made the old loop
            // fail open.
            let now = Instant::now();
            if now < deadline { thread::sleep(deadline - now); }
            // Re-read each tick so the GUI can retune the rate live without a
            // restart. Relaxed is sufficient: a tick either side of the change
            // is immaterial, and this path must never take the metrics mutex.
            let next_period = tick_period(target_fps.load(Ordering::Relaxed));
            if next_period != period { period = next_period; deadline = Instant::now(); }
            deadline = next_deadline(deadline, Instant::now(), period);
        }
    });
}
