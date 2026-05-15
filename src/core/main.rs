#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::{Context, Result};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use std::env;
use std::fs;
use std::path::Path;
use crossbeam_channel::{unbounded, bounded, select};
use tray_icon::menu::MenuEvent;
use simplelog::{WriteLogger, TermLogger, Config as LogConfig, LevelFilter};
use xcb::x;

use crate::core::config::Config;
use crate::core::window::{create_all_windows, find_keycode, grab_key_combinations};
use crate::metrics::{MetricsCommand, spawn_metrics_thread, MetricId, MetricValue};
use crate::render::Renderer;
use crate::core::{layout, logging, version, productivity};
use crate::build_logger;
use crate::ui::tray::{SystemTray, MENU_QUIT_ID, MENU_RELOAD_ID, MENU_CONFIG_GUI_ID};
use crate::ui::gui::{GuiEvent, ConfigWindow};

pub fn run() -> Result<()> {
    // 1. Load Config First
    let mut config = Config::load().context("Failed to load configuration")?;
    
    // 2. Init Logger
    version::print_startup_info();
    
    if env::args().any(|a| a == "debug-build") {
        build_logger::log_build_event("cargo build --release", &config.logging.log_path);
        return Ok(());
    }

    if config.logging.enabled {
        let log_dir = std::path::Path::new(&config.logging.log_path);
        if !log_dir.exists() {
            fs::create_dir_all(log_dir).context("Failed to create log directory")?;
        }
        
        let _ = WriteLogger::init(
            LevelFilter::Info,
            LogConfig::default(),
            fs::File::create(log_dir.join("matrix_overlay.log")).context("Failed to create log file")?
        );
        println!("Logging enabled. Directory: {}", config.logging.log_path);
    } else {
        env_logger::init();
    }
    log::info!("Initializing Matrix Overlay... v0.1.3-FORCE_REBUILD");

    config.cosmetics.rain_mode = "fall".to_string();
    config.cosmetics.realism_scale = 8;

    log::info!("Configuration loaded successfully.");

    // 3. Spawn Metrics Thread
    let (metrics, shutdown, _metrics_handle, metrics_tx) = spawn_metrics_thread(&config);

    // 4. Setup XCB Connection
    let (conn, screen_num) = xcb::Connection::connect(None).context("Failed to connect to X server")?;
    let conn = Arc::new(conn);

    log::info!("Connected to XCB. Screen: {}", screen_num);

    // 6. Set Background
    log::info!("Setting background to black...");
    // **[HARDENING: Secure Execution]**
    // Using safe_exec to avoid shell interpretation.
    if let Err(e) = safe_exec("xsetroot", &["-solid", "#000000"]) {
        log::warn!("Failed to execute xsetroot: {}", e);
    }

    // 5c. Setup Hotkeys
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).context("No screen found")?;
    let root = screen.root();

    let keycode_w = find_keycode(&conn, 0x0077)?.context("Could not find keycode for 'w'")?;
    grab_key_combinations(&conn, root, keycode_w, x::ModMask::CONTROL | x::ModMask::N1)?;

    let keycode_q = find_keycode(&conn, 0x0071)?.context("Could not find keycode for 'q'")?;
    grab_key_combinations(&conn, root, keycode_q, x::ModMask::CONTROL | x::ModMask::N1)?;

    conn.flush()?;
    log::info!("Grabbed hotkeys: Ctrl+Alt+W (Toggle), Ctrl+Alt+Q (Quit)");

    if env::args().any(|a| a == "--test-layering") {
        log::info!("Test Mode: Layering Verification active. Sleeping 10s...");
        thread::sleep(Duration::from_secs(10));
        return Ok(());
    }

    if let Err(e) = setup_autostart() {
        log::warn!("Failed to setup autostart: {}", e);
    }

    #[cfg(target_os = "linux")]
    {
        if let Err(e) = gtk::init() {
            log::warn!("Failed to initialize GTK: {}", e);
        }
    }

    let _tray = match SystemTray::new(&config) {
        Ok(t) => Some(t),
        Err(e) => {
            log::warn!("Failed to initialize system tray: {}", e);
            None
        }
    };

    // Channel for XCB events
    let (xcb_tx, xcb_rx_overlay) = unbounded();
    let conn_event = conn.clone();
    thread::spawn(move || {
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

    let (interval_tx, _interval_rx) = unbounded::<Duration>();
    let (gui_tx, gui_rx) = unbounded::<GuiEvent>();
    let (control_tx, control_rx) = unbounded::<GuiEvent>();
    
    let config_arc = Arc::new(config.clone());
    let conn_arc = Arc::clone(&conn);
    let shutdown_arc = Arc::clone(&shutdown);
    let metrics_arc = Arc::clone(&metrics);

    // 8. Spawn Overlay Thread
    let control_tx_overlay = control_tx.clone();
    let interval_tx_overlay = interval_tx.clone();
    let metrics_tx_overlay = metrics_tx.clone();

    thread::spawn(move || {
        log::info!("Overlay logic thread started.");
        let mut config_overlay = (*config_arc).clone();

        let wm = match create_all_windows(&conn_arc, &config_overlay) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to create windows: {}", e);
                return;
            }
        };

        let mut renderers = Vec::new();
        for (i, ctx) in wm.monitors.iter().enumerate() {
            let layout = layout::compute(&config_overlay, i, ctx.monitor.width, ctx.monitor.height);
            if let Ok(renderer) = Renderer::new(ctx.monitor.width, ctx.monitor.height, i, layout, &config_overlay) {
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

        let keycode_w = find_keycode(&conn_arc, 0x0077).unwrap_or(Some(0)).unwrap_or(0);
        let keycode_q = find_keycode(&conn_arc, 0x0071).unwrap_or(Some(0)).unwrap_or(0);
        let mut visible = true;
        let mut last_draw = Instant::now();

        loop {
            if shutdown_arc.load(Ordering::Relaxed) { break; }

            select! {
                recv(xcb_rx_overlay) -> event_res => {
                    if let Ok(event) = event_res {
                        match event {
                            xcb::Event::X(x::Event::KeyPress(ev)) => {
                                if ev.detail() == keycode_w {
                                    visible = !visible;
                                    for ctx in &wm.monitors {
                                        if visible { let _ = conn_arc.send_request(&x::MapWindow { window: ctx.window }); }
                                        else { let _ = conn_arc.send_request(&x::UnmapWindow { window: ctx.window }); }
                                    }
                                    let _ = conn_arc.flush();
                                } else if ev.detail() == keycode_q {
                                    shutdown_arc.store(true, Ordering::Relaxed);
                                    break;
                                }
                            },
                            xcb::Event::X(x::Event::Expose(ev)) => {
                                if visible {
                                    if let Some(idx) = wm.monitors.iter().position(|m| m.window == ev.window()) {
                                        if let Some(renderer) = renderers.get_mut(idx) {
                                            if let Ok(shared) = metrics_arc.lock() {
                                                let dt = last_draw.elapsed();
                                                let _ = renderer.draw(&conn_arc, ev.window(), &config_overlay, &shared.data, dt);
                                            }
                                        }
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                },
                recv(tick_thread_rx) -> _ => {
                    if visible {
                        let dt = last_draw.elapsed();
                        last_draw = Instant::now();
                        if let Ok(mut shared) = metrics_arc.lock() {
                            // **[NEW: Coordinate Anchoring]**
                            // Check if a fresh location was detected by the metrics collectors.
                            if let Some(MetricValue::Location(lat, lon)) = shared.data.values.remove(&MetricId::LocationData) {
                                if config_overlay.weather.lat != lat || config_overlay.weather.lon != lon {
                                    log::info!("[Security] Anchoring newly detected location ({}, {}) to config.json", lat, lon);
                                    config_overlay.weather.lat = lat;
                                    config_overlay.weather.lon = lon;
                                    config_overlay.weather.auto_location = false; // Disable auto-lookup once anchored
                                    let _ = config_overlay.save();
                                }
                            }

                            for (i, renderer) in renderers.iter_mut().enumerate() {
                                if let Some(ctx) = wm.monitors.get(i) {
                                    let _ = renderer.draw(&conn_arc, ctx.window, &config_overlay, &shared.data, dt);
                                }
                            }
                        }
                    }
                },
                recv(MenuEvent::receiver()) -> event_res => {
                    if let Ok(event) = event_res {
                        if event.id.as_ref() == MENU_QUIT_ID {
                            shutdown_arc.store(true, Ordering::Relaxed);
                            break;
                        }
                        if event.id.as_ref() == MENU_RELOAD_ID {
                            if let Ok(new_config) = Config::load() {
                                config_overlay = new_config.clone();
                                let _ = interval_tx_overlay.send(Duration::from_millis(config_overlay.general.update_ms));
                                for renderer in &mut renderers { renderer.update_config(config_overlay.clone()); }
                                let _ = metrics_tx_overlay.send(MetricsCommand::UpdateConfig(config_overlay.clone()));
                            }
                        }
                        if event.id.as_ref() == MENU_CONFIG_GUI_ID {
                            let _ = control_tx_overlay.send(GuiEvent::OpenConfig);
                        }
                    }
                },
                recv(gui_rx) -> event_res => {
                    if let Ok(event) = event_res {
                        match event {
                            GuiEvent::Reload => {
                                if let Ok(new_config) = Config::load() {
                                    config_overlay = new_config.clone();
                                    let _ = interval_tx_overlay.send(Duration::from_millis(config_overlay.general.update_ms));
                                    for renderer in &mut renderers { renderer.update_config(config_overlay.clone()); }
                                    let _ = metrics_tx_overlay.send(MetricsCommand::UpdateConfig(config_overlay.clone()));
                                }
                            },
                            GuiEvent::PurgeLogs => {
                                let _ = logging::Logger::purge_debug_logs("/tmp/matrix_overlay_logs");
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
        let _ = wm.cleanup(&conn_arc);
    });

    // 7c. Spawn Productivity Thread
    let productivity_config = config.clone();
    let productivity_shutdown = shutdown.clone();
    thread::spawn(move || {
        log::info!("Productivity thread started.");
        let mut last_check = Instant::now();
        while !productivity_shutdown.load(Ordering::Relaxed) {
            if last_check.elapsed() >= Duration::from_secs(3600) {
                last_check = Instant::now();
                let _ = productivity::run_auto_commit_cycle(&productivity_config);
            }
            thread::sleep(Duration::from_secs(60));
        }
    });

    // Start GTK Main Loop
    #[cfg(target_os = "linux")]
    {
        loop {
            if shutdown.load(Ordering::Relaxed) { break; }
            while gtk::events_pending() { gtk::main_iteration(); }
            
            while let Ok(event) = control_rx.try_recv() {
                match event {
                    GuiEvent::OpenConfig => {
                        if let Ok(new_config) = Config::load() {
                            let window = ConfigWindow::new(new_config, gui_tx.clone());
                            window.show();
                        }
                    },
                    _ => {}
                }
            }
            thread::sleep(Duration::from_millis(16));
        }
    }

    shutdown.store(true, Ordering::Relaxed);
    Ok(())
}

fn setup_autostart() -> Result<()> {
    let home = env::var("HOME").context("HOME not set")?;
    let autostart_dir = Path::new(&home).join(".config/autostart");
    if !autostart_dir.exists() { fs::create_dir_all(&autostart_dir)?; }
    
    let desktop_file = autostart_dir.join("matrix-overlay.desktop");

    // **[HARDENING: Symlink Validation]**
    // Ensure we don't follow a malicious symlink to a sensitive system file.
    if desktop_file.exists() {
        let metadata = fs::symlink_metadata(&desktop_file)?;
        if metadata.file_type().is_symlink() {
            log::warn!("Security Alert: Autostart file is a symlink. Removing for safety.");
            let _ = fs::remove_file(&desktop_file);
        }
    }

    if !desktop_file.exists() {
        let current_exe = env::current_exe()?;
        let content = format!(
            "[Desktop Entry]\nType=Application\nName=Matrix Overlay\nExec={}\nX-GNOME-Autostart-enabled=true\n",
            current_exe.to_string_lossy()
        );
        fs::write(&desktop_file, content)?;

        // **[HARDENING: Permissions]**
        // Ensure the desktop file is only writable by the user (0644).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&desktop_file, fs::Permissions::from_mode(0o644));
        }
    }
    Ok(())
}

/// **[HARDENING: Secure Execution]**
/// Executes a command without shell interpretation, ensuring arguments cannot be escaped.
fn safe_exec(cmd: &str, args: &[&str]) -> Result<()> {
    let status = std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .context(format!("Failed to execute {}", cmd))?;
    
    if !status.success() {
        log::warn!("Command {} failed with status {}", cmd, status);
    }
    Ok(())
}