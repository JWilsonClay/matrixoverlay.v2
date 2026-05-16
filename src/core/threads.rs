// src/core/threads.rs
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
use crate::ui::tray::{SystemTray, MENU_QUIT_ID, MENU_RELOAD_ID, MENU_CONFIG_GUI_ID, MENU_UPDATE_ID};
use crate::core::update::UpdateEvent;

pub fn spawn_xcb_thread(conn: Arc<xcb::Connection>, xcb_tx: Sender<xcb::Event>) {
    thread::spawn(move || {
        let conn_event = Arc::clone(&conn);
        loop {
            match conn_event.wait_for_event() {
                Ok(event) => {
                    if xcb_tx.send(event).is_err() { break; }
                }
                Err(e) => {
                    log::error!("XCB Connection Error: {}", e);
                    break; 
                }
            }
        }
    });
}

pub fn spawn_productivity_thread(config: Config, shutdown: Arc<AtomicBool>) {
    thread::spawn(move || {
        log::info!("Productivity thread started.");
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
        log::info!("Overlay logic thread started.");
        let mut config_overlay = config.clone();

        let wm = match create_all_windows(&conn, &config_overlay) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to create windows: {}", e);
                return;
            }
        };

        let mut renderers = Vec::new();
        for (i, ctx) in wm.monitors.iter().enumerate() {
            let layout = layout::compute(&config_overlay, i, 1920, 1080); // Width/Height placeholders
            if let Ok(renderer) = Renderer::new(1920, 1080, i, layout, &config_overlay) {
                renderers.push(renderer);
            }
        }
        
        let (tick_thread_tx, tick_thread_rx) = bounded(1);
        thread::spawn(move || {
            let interval = Duration::from_millis(33); 
            loop {
                let start = Instant::now();
                if tick_thread_tx.send(()).is_err() { break; }
                let elapsed = start.elapsed();
                if elapsed < interval { thread::sleep(interval - elapsed); }
                else { thread::sleep(Duration::from_millis(1)); }
            }
        });

        let keycode_w = find_keycode(&conn, 0x0077).unwrap_or(Some(0)).unwrap_or(0);
        let keycode_q = find_keycode(&conn, 0x0071).unwrap_or(Some(0)).unwrap_or(0);
        let mut visible = true;
        let mut last_draw = Instant::now();

        loop {
            if shutdown.load(Ordering::Relaxed) { break; }

            select! {
                recv(xcb_rx) -> event_res => {
                    if let Ok(event) = event_res {
                        handle_xcb_event(event, &conn, &wm, &mut visible, &metrics, &mut renderers, &config_overlay, &mut last_draw, keycode_w, keycode_q, &shutdown);
                    }
                },
                recv(tick_thread_rx) -> _ => {
                    if visible {
                        draw_frame(&conn, &wm, &mut renderers, &config_overlay, &metrics, &mut last_draw);
                    }
                },
                recv(MenuEvent::receiver()) -> event_res => {
                    if let Ok(event) = event_res {
                        handle_menu_event(event, &mut config_overlay, &mut renderers, &metrics_tx, &control_tx, &shutdown);
                    }
                },
                recv(gui_rx) -> event_res => {
                    if let Ok(event) = event_res {
                        match event {
                            GuiEvent::Reload => {
                                if let Ok(new_config) = Config::load() {
                                    config_overlay = new_config.clone();
                                    for renderer in &mut renderers { renderer.update_config(config_overlay.clone()); }
                                    let _ = metrics_tx.send(MetricsCommand::UpdateConfig(config_overlay.clone()));
                                }
                            },
                            GuiEvent::PurgeLogs => {
                                let _ = logging::Logger::purge_debug_logs("/tmp/matrix_overlay_logs");
                            },
                            _ => {}
                        }
                    }
                },
                recv(update_rx) -> event_res => {
                    if let Ok(event) = event_res {
                        if let UpdateEvent::UpdateAvailable { version, .. } = event {
                            log::info!("[Update] New version available: v{}", version);
                            SystemTray::show_update_bubble(&version);
                            let _ = control_tx.send(GuiEvent::UpdateAvailable(version));
                        }
                    }
                }
            }
        }
        let _ = wm.cleanup(&conn);
    });
}

fn handle_xcb_event(
    event: xcb::Event,
    conn: &Arc<xcb::Connection>,
    wm: &crate::core::window::WindowManager,
    visible: &mut bool,
    metrics: &Arc<std::sync::Mutex<SharedMetrics>>,
    renderers: &mut Vec<Renderer>,
    config: &Config,
    last_draw: &mut Instant,
    keycode_w: u8,
    keycode_q: u8,
    shutdown: &Arc<AtomicBool>,
) {
    use xcb::x;
    match event {
        xcb::Event::X(x::Event::KeyPress(ev)) => {
            if ev.detail() == keycode_w {
                *visible = !*visible;
                for ctx in &wm.monitors {
                    if *visible { let _ = conn.send_request(&x::MapWindow { window: ctx.window }); }
                    else { let _ = conn.send_request(&x::UnmapWindow { window: ctx.window }); }
                }
                let _ = conn.flush();
            } else if ev.detail() == keycode_q {
                shutdown.store(true, Ordering::Relaxed);
            }
        },
        xcb::Event::X(x::Event::Expose(ev)) => {
            if *visible {
                if let Some(idx) = wm.monitors.iter().position(|m| m.window == ev.window()) {
                    if let Some(renderer) = renderers.get_mut(idx) {
                        if let Ok(shared) = metrics.lock() {
                            let dt = last_draw.elapsed();
                            let _ = renderer.draw(conn, ev.window(), config, &shared.data, dt);
                        }
                    }
                }
            }
        },
        _ => {}
    }
}

fn draw_frame(
    conn: &Arc<xcb::Connection>,
    wm: &crate::core::window::WindowManager,
    renderers: &mut Vec<Renderer>,
    config: &Config,
    metrics: &Arc<std::sync::Mutex<SharedMetrics>>,
    last_draw: &mut Instant,
) {
    let dt = last_draw.elapsed();
    *last_draw = Instant::now();
    if let Ok(shared) = metrics.lock() {
        for (i, renderer) in renderers.iter_mut().enumerate() {
            if let Some(ctx) = wm.monitors.get(i) {
                let _ = renderer.draw(conn, ctx.window, config, &shared.data, dt);
            }
        }
    }
}

fn handle_menu_event(
    event: MenuEvent,
    config: &mut Config,
    renderers: &mut Vec<Renderer>,
    metrics_tx: &Sender<MetricsCommand>,
    control_tx: &Sender<GuiEvent>,
    shutdown: &Arc<AtomicBool>,
) {
    if event.id.as_ref() == MENU_QUIT_ID {
        shutdown.store(true, Ordering::Relaxed);
    } else if event.id.as_ref() == MENU_RELOAD_ID {
        if let Ok(new_config) = Config::load() {
            *config = new_config.clone();
            for renderer in renderers { renderer.update_config(config.clone()); }
            let _ = metrics_tx.send(MetricsCommand::UpdateConfig(config.clone()));
        }
    } else if event.id.as_ref() == MENU_CONFIG_GUI_ID {
        let _ = control_tx.send(GuiEvent::OpenConfig);
    } else if event.id.as_ref() == MENU_UPDATE_ID {
        log::info!("[Update] User triggered update...");
    }
}
