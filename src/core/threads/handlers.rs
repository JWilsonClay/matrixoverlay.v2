// src/core/threads/handlers.rs
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use crossbeam_channel::Sender;
use tray_icon::menu::MenuEvent;
use crate::core::config::Config;
use crate::metrics::{MetricsCommand, SharedMetrics};
use crate::render::Renderer;
use crate::ui::gui::GuiEvent;
use crate::ui::tray::{SystemTray, MENU_QUIT_ID, MENU_RELOAD_ID, MENU_CONFIG_GUI_ID, MENU_UPDATE_ID};
use crate::core::update::UpdateEvent;

pub fn handle_xcb_event(
    event: xcb::Event, conn: &Arc<xcb::Connection>, wm: &crate::core::window::WindowManager,
    visible: &mut bool, metrics: &Arc<std::sync::Mutex<SharedMetrics>>, renderers: &mut Vec<Renderer>,
    config: &Config, last_draw: &mut Instant, key_w: u8, key_q: u8, shutdown: &Arc<AtomicBool>,
) {
    use xcb::x;
    match event {
        xcb::Event::X(x::Event::KeyPress(ev)) => {
            if ev.detail() == key_w {
                *visible = !*visible;
                for ctx in &wm.monitors {
                    let _ = if *visible { conn.send_request(&x::MapWindow { window: ctx.window }) }
                            else { conn.send_request(&x::UnmapWindow { window: ctx.window }) };
                }
                let _ = conn.flush();
            } else if ev.detail() == key_q { shutdown.store(true, Ordering::SeqCst); }
        },
        xcb::Event::X(x::Event::Expose(ev)) => {
            if *visible {
                if let Some(idx) = wm.monitors.iter().position(|m| m.window == ev.window()) {
                    if let (Some(r), Ok(s)) = (renderers.get_mut(idx), metrics.lock()) {
                        let _ = r.draw(conn, ev.window(), config, &s.data, last_draw.elapsed());
                    }
                }
            }
        },
        _ => {}
    }
}

pub fn draw_frame(
    conn: &Arc<xcb::Connection>, wm: &crate::core::window::WindowManager,
    renderers: &mut Vec<Renderer>, config: &Config, metrics: &Arc<std::sync::Mutex<SharedMetrics>>,
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

pub fn handle_menu_event(
    event: MenuEvent, config: &mut Config, renderers: &mut Vec<Renderer>,
    metrics_tx: &Sender<MetricsCommand>, control_tx: &Sender<GuiEvent>, shutdown: &Arc<AtomicBool>,
) {
    let id = event.id.as_ref();
    if id == MENU_QUIT_ID { shutdown.store(true, Ordering::SeqCst); }
    else if id == MENU_RELOAD_ID {
        if let Ok(new_cfg) = Config::load() {
            *config = new_cfg.clone();
            for r in renderers { r.update_config(config.clone()); }
            let _ = metrics_tx.send(MetricsCommand::UpdateConfig(config.clone()));
        }
    } else if id == MENU_CONFIG_GUI_ID { let _ = control_tx.send(GuiEvent::OpenConfig); }
    else if id == MENU_UPDATE_ID { log::info!("[Update] User triggered update..."); }
}

pub fn handle_gui_event(
    event: GuiEvent, config: &mut Config, renderers: &mut Vec<Renderer>, metrics_tx: &Sender<MetricsCommand>,
) {
    match event {
        GuiEvent::Reload => {
            if let Ok(new_cfg) = Config::load() {
                *config = new_cfg.clone();
                for r in renderers { r.update_config(config.clone()); }
                let _ = metrics_tx.send(MetricsCommand::UpdateConfig(config.clone()));
            }
        },
        GuiEvent::PurgeLogs => {
            // [HARDENING] Use configured path, not hardcoded /tmp
            let _ = crate::core::logging::Logger::purge_debug_logs(&config.logging.log_path);
        },
        _ => {}
    }
}

pub fn handle_update_event(event: UpdateEvent, control_tx: &Sender<GuiEvent>) {
    if let UpdateEvent::UpdateAvailable { version, .. } = event {
        SystemTray::show_update_bubble(&version);
        let _ = control_tx.send(GuiEvent::UpdateAvailable(version));
    }
}
