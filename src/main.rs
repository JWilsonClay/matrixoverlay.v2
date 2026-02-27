#![allow(dead_code)]
#![allow(unused_imports)]

mod config;
mod layout;
mod metrics;
mod render;
mod tray;
mod window;

use anyhow::{Context, Result};
use std::sync::{Arc, atomic::Ordering};
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::env;
use std::fs;
use std::path::Path;
use crossbeam_channel::{unbounded, bounded, select};
use tray_icon::menu::MenuEvent;
use xcb::x;

use crate::config::Config;
use crate::window::create_all_windows;
use crate::metrics::{MetricData, MetricId, MetricValue, MetricsCommand, spawn_metrics_thread};
use crate::render::Renderer;
use crate::layout::Layout;
use crate::tray::{SystemTray, MENU_QUIT_ID, MENU_RELOAD_ID, MENU_EDIT_ID};

fn main() -> Result<()> {
    // 1. Init env_logger
    env_logger::init();
    log::info!("Initializing Matrix Overlay...");

    // 2. Load Config
    let mut config = Config::load().context("Failed to load configuration")?;
    log::info!("Configuration loaded successfully.");
    for (i, screen) in config.screens.iter().enumerate() {
        log::info!("Monitor {}: Configured metrics: {:?}", i, screen.metrics);
    }

    // Verify Privacy Settings
    if config.weather.enabled {
        log::info!("Weather enabled (Lat: {}, Lon: {})", config.weather.lat, config.weather.lon);
    } else {
        log::info!("Weather disabled (Privacy Mode active)");
    }

    // 3. Spawn Metrics Thread
    let (metrics, shutdown, _metrics_handle, metrics_tx) = spawn_metrics_thread(&config);

    // 4. Setup XCB Connection
    let (conn, screen_num) = xcb::Connection::connect(None).context("Failed to connect to X server")?;
    let conn = Arc::new(conn); // Wrap in Arc for sharing with event thread

    log::info!("Connected to XCB. Screen: {}", screen_num);

    // 5. Create Windows
    let wm = create_all_windows(&conn, &config).context("Failed to create windows")?;

    log::info!("Created {} overlay windows.", wm.monitors.len());
    for (i, ctx) in wm.monitors.iter().enumerate() {
        log::info!("  Window {}: ID={:?}, Monitor={}", i, ctx.window, ctx.monitor.name);
    }

    // 5b. Initialize Renderers
    let mut renderers = Vec::new();
    for (i, ctx) in wm.monitors.iter().enumerate() {
        let screen_config = config.screens.get(i).or(config.screens.first());
        
        let layout = if let Some(screen) = screen_config {
            layout::compute(screen, ctx.monitor.width, ctx.monitor.height, config.general.font_size as f64)
        } else {
            Layout { items: Vec::new() }
        };

        let renderer = Renderer::new(ctx.monitor.width, ctx.monitor.height, i, layout, &config)?;
        renderers.push(renderer);
    }

    // 6. Set Background
    log::info!("Setting background to black...");
    if let Err(e) = Command::new("xsetroot")
        .args(&["-solid", "#000000"])
        .spawn() 
    {
        log::warn!("Failed to execute xsetroot: {}", e);
    }

    // 5c. Setup Hotkey (Ctrl+Alt+W)
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).context("No screen found")?;
    let root = screen.root();

    // 'w' keysym is 0x0077
    let keycode_w = find_keycode(&conn, 0x0077)?.context("Could not find keycode for 'w'")?;
    
    grab_key_combinations(&conn, root, keycode_w, x::ModMask::CONTROL | x::ModMask::N1)?;

    // 'q' keysym is 0x0071
    let keycode_q = find_keycode(&conn, 0x0071)?.context("Could not find keycode for 'q'")?;

    grab_key_combinations(&conn, root, keycode_q, x::ModMask::CONTROL | x::ModMask::N1)?;

    conn.flush()?;
    log::info!("Grabbed hotkeys: Ctrl+Alt+W (Toggle), Ctrl+Alt+Q (Quit)");

    // 7. Test Mode Check
    if env::args().any(|a| a == "--test-layering") {
        log::info!("Test Mode: Layering Verification active.");
        log::info!("Windows created. Sleeping for 10s to allow manual 'xprop' or 'xwininfo' checks...");
        thread::sleep(Duration::from_secs(10));
        log::info!("Test Mode complete. Exiting.");
        return Ok(());
    }

    // 7a. Setup Autostart
    if let Err(e) = setup_autostart() {
        log::warn!("Failed to setup autostart: {}", e);
    }

    // 7b. Initialize GTK (Required for Tray Icon on Linux)
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = gtk::init() {
            log::warn!("Failed to initialize GTK: {}", e);
        }
    }

    // 7b. Initialize System Tray
    let _tray = match SystemTray::new() {
        Ok(t) => Some(t),
        Err(e) => {
            log::warn!("Failed to initialize system tray: {}", e);
            None
        }
    };

    // 8. Event Loop Setup
    log::info!("Entering event loop...");
    
    // Channel for XCB events (Threaded Poller)
    let (xcb_tx, xcb_rx) = unbounded();
    let conn_event = conn.clone();
    thread::spawn(move || {
        loop {
            match conn_event.wait_for_event() {
                Ok(event) => {
                    if xcb_tx.send(event).is_err() { break; }
                }
                Err(e) => {
                    log::error!("XCB Wait Error: {}", e);
                    break; 
                }
            }
        }
    });

    // Channel for Redraw Ticks
    let (tick_tx, tick_rx) = bounded(1);
    let update_ms = config.general.update_ms;
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(update_ms));
            if tick_tx.send(()).is_err() { break; }
        }
    });

    let mut visible = true;
    let mut first_redraw = true;

    loop {
        // Pump GTK events (for Tray Icon)
        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        select! {
            recv(xcb_rx) -> event_res => {
                if let Ok(event) = event_res {
                    match event {
                        xcb::Event::X(x::Event::KeyPress(ev)) => {
                            log::info!("KeyPress received: keycode={}, state={:?}", ev.detail(), ev.state());
                            if ev.detail() == keycode_w {
                                log::info!("Hotkey activated. Toggling visibility.");
                                visible = !visible;
                                for ctx in &wm.monitors {
                                    if visible {
                                        conn.send_request(&x::MapWindow { window: ctx.window });
                                    } else {
                                        conn.send_request(&x::UnmapWindow { window: ctx.window });
                                    }
                                }
                                conn.flush()?;
                            } else if ev.detail() == keycode_q {
                                log::info!("Hotkey Ctrl+Alt+Q activated. Exiting.");
                                break;
                            }
                        },
                        xcb::Event::X(x::Event::Expose(ev)) => {
                            if visible {
                                // Find renderer for this window and redraw
                                if let Some(idx) = wm.monitors.iter().position(|m| m.window == ev.window()) {
                                    if let Some(renderer) = renderers.get_mut(idx) {
                                        if let Ok(shared) = metrics.lock() {
                                            let _ = renderer.draw(&conn, ev.window(), &config, &shared.data);
                                        }
                                    }
                                }
                            }
                        },
                        _ => {}
                    }
                } else {
                    break; // Channel closed
                }
            },
            recv(tick_rx) -> _ => {
                if visible {
                    if let Ok(shared) = metrics.lock() {
                        if first_redraw {
                            log::info!("First redraw triggered. Data: {}", shared.data.summary());
                            first_redraw = false;
                        }

                        for (i, renderer) in renderers.iter_mut().enumerate() {
                            if let Some(ctx) = wm.monitors.get(i) {
                                log::debug!("Redrawing Window {} [{}x{} @ {},{}]. Metrics: {}", 
                                    i, ctx.monitor.width, ctx.monitor.height, ctx.monitor.x, ctx.monitor.y,
                                    shared.data.values.len());

                                if let Err(e) = renderer.draw(&conn, ctx.window, &config, &shared.data) {
                                    log::error!("Render failed on monitor {}: {}", i, e);
                                }
                            }
                        }
                    }
                }
            },
            recv(MenuEvent::receiver()) -> event_res => {
                if let Ok(event) = event_res {
                    if event.id.as_ref() == MENU_QUIT_ID {
                        log::info!("Quit requested via Tray.");
                        break;
                    }
                    if event.id.as_ref() == MENU_RELOAD_ID {
                        log::info!("Reloading configuration...");
                        match Config::load() {
                            Ok(new_config) => {
                                config = new_config.clone();
                                
                                // Update all renderers
                                for renderer in &mut renderers {
                                    renderer.update_config(new_config.clone());
                                }
                                
                                // Update metrics thread
                                if let Err(e) = metrics_tx.send(MetricsCommand::UpdateConfig(new_config.clone())) {
                                    log::error!("Failed to notify metrics thread of reload: {}", e);
                                }
                                
                                log::info!("Config reloaded and broadcast to all modules.");
                            },
                            Err(e) => log::error!("Failed to reload config: {}", e),
                        }
                    }
                    if event.id.as_ref() == "about" {
                        log::info!("Displaying About info...");
                        println!("Matrix Overlay v2 - jwils (John Wilson) and Grok (xAI)");
                        // NOTE: Open GUI notification in Stage 4/5 integration
                    }
                    if event.id.as_ref() == MENU_EDIT_ID {
                        if let Ok(home) = env::var("HOME") {
                            let _ = Command::new("xdg-open").arg(format!("{}/.config/matrix-overlay/config.json", home)).spawn();
                        }
                    }
                }
            }
        }
    }

    log::info!("Shutting down...");
    
    // Ungrab key
    let _ = conn.send_request(&x::UngrabKey { key: keycode_w, grab_window: root, modifiers: x::ModMask::ANY });
    let _ = conn.send_request(&x::UngrabKey { key: keycode_q, grab_window: root, modifiers: x::ModMask::ANY });
    let _ = conn.flush();

    shutdown.store(true, Ordering::Relaxed);
    wm.cleanup(&conn)?;

    Ok(())
}

fn setup_autostart() -> Result<()> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    let autostart_dir = Path::new(&home).join(".config/autostart");
    if !autostart_dir.exists() {
        fs::create_dir_all(&autostart_dir).context("Failed to create autostart directory")?;
    }
    
    let desktop_file = autostart_dir.join("matrix-overlay.desktop");
    if !desktop_file.exists() {
        let current_exe = env::current_exe().context("Failed to get current executable path")?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Matrix Overlay\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            current_exe.to_string_lossy()
        );
        fs::write(&desktop_file, content).context("Failed to write desktop file")?;
        log::info!("Created autostart entry at {:?}", desktop_file);
    }
    Ok(())
}

fn find_keycode(conn: &xcb::Connection, keysym: u32) -> Result<Option<u8>> {
    let setup = conn.get_setup();
    let min_keycode = setup.min_keycode();
    let max_keycode = setup.max_keycode();
    let count = max_keycode - min_keycode + 1;

    let cookie = conn.send_request(&x::GetKeyboardMapping {
        first_keycode: min_keycode,
        count,
    });
    let reply = conn.wait_for_reply(cookie)?;
    
    let keysyms = reply.keysyms();
    let keysyms_per_keycode = reply.keysyms_per_keycode() as usize;

    for (i, &sym) in keysyms.iter().enumerate() {
        if sym == keysym {
            let keycode_offset = i / keysyms_per_keycode;
            let keycode = min_keycode as usize + keycode_offset;
            return Ok(Some(keycode as u8));
        }
    }
    Ok(None)
}

fn grab_key_combinations(conn: &xcb::Connection, root: x::Window, keycode: u8, base_mods: x::ModMask) -> Result<()> {
    // Grab with CapsLock (LOCK) and NumLock (M2) combinations to ensure hotkey works in all states
    let modifiers = [
        base_mods,
        base_mods | x::ModMask::LOCK,
        base_mods | x::ModMask::N2,
        base_mods | x::ModMask::LOCK | x::ModMask::N2,
    ];

    for &mods in &modifiers {
        conn.send_request(&x::GrabKey {
            owner_events: true,
            grab_window: root,
            modifiers: mods,
            key: keycode,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        });
    }
    Ok(())
}