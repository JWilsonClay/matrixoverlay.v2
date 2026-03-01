#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::{bail, Context, Result};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use std::env;
use std::fs;
use std::path::Path;
use git2::Repository;
use crossbeam_channel::{unbounded, bounded, select, after, Receiver};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use simplelog::{WriteLogger, TermLogger, Config as LogConfig, LevelFilter, TerminalMode, ColorChoice};
use chrono::Local;
use xcb::x;

use matrix_overlay::config::Config;
use matrix_overlay::window::create_all_windows;
use matrix_overlay::metrics::{MetricData, MetricId, MetricValue, MetricsCommand, spawn_metrics_thread};
use matrix_overlay::render::Renderer;
use matrix_overlay::layout::{self, Layout};
use matrix_overlay::logging;
use matrix_overlay::version;
use matrix_overlay::build_logger;
use matrix_overlay::path_utils;
use matrix_overlay::tray::{SystemTray, MENU_QUIT_ID, MENU_RELOAD_ID, MENU_EDIT_ID, MENU_THEME_CLASSIC, MENU_THEME_CALM, MENU_THEME_ALERT, MENU_TOGGLE_AUTO_COMMIT, MENU_TOGGLE_OLLAMA, MENU_CONFIG_GUI_ID, MENU_CONFIG_JSON_ID};
use matrix_overlay::gui::{GuiEvent, ConfigWindow};

fn main() -> Result<()> {
    // 1. Load Config First (to determine logging)
    let mut config = Config::load().context("Failed to load configuration")?;
    
    // 2. Init Logger
    version::print_startup_info();
    
    // Check for debug-build subcommand
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

    // FORCE OVERRIDE: Ensure rain is enabled for verification
    config.cosmetics.rain_mode = "fall".to_string();
    // FORCE OVERRIDE: Max density to ensure visibility (Fixes "No streams to draw")
    config.cosmetics.realism_scale = 8;

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

    // 5. Create Windows & Initialize Renderers - MOVED TO BACKGROUND THREAD

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
    let _tray = match SystemTray::new(&config) {
        Ok(t) => Some(t),
        Err(e) => {
            log::warn!("Failed to initialize system tray: {}", e);
            None
        }
    };

    // Channel for XCB events (Threaded Poller)
    let (xcb_tx, xcb_rx_overlay) = unbounded();
    let conn_event = conn.clone();
    thread::spawn(move || {
        loop {
            match conn_event.wait_for_event() {
                Ok(event) => {
                    if xcb_tx.send(event).is_err() { break; }
                }
                Err(xcb::Error::Protocol(e)) => {
                    log::warn!("XCB Protocol Error (Ignored): {:?}", e);
                }
                Err(e) => {
                    log::error!("XCB Connection Error: {}", e);
                    break; 
                }
            }
        }
    });

    let (interval_tx, interval_rx) = unbounded::<Duration>();
    let (gui_tx, gui_rx) = unbounded::<GuiEvent>();
    let (control_tx, control_rx) = unbounded::<GuiEvent>();
    
    // ARC for sharing across threads
    let config_arc = Arc::new(config.clone());
    let conn_arc = Arc::clone(&conn);
    let shutdown_arc = Arc::clone(&shutdown);
    let metrics_arc = Arc::clone(&metrics);

    // 8. Spawn Overlay Thread
    let gui_tx_pass = gui_tx.clone();
    let control_tx_overlay = control_tx.clone();
    let interval_tx_overlay = interval_tx.clone();
    let metrics_tx_overlay = metrics_tx.clone();
    let menu_channel = MenuEvent::receiver();

    thread::spawn(move || {
        log::info!("Overlay logic thread started.");
        let mut config_overlay = (*config_arc).clone();

        // Initialize Windows and Renderers within this thread (to avoid Cairo thread-safety issues)
        let wm = match create_all_windows(&conn_arc, &config_overlay) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to create windows in background thread: {}", e);
                return;
            }
        };

        let mut renderers = Vec::new();
        for (i, ctx) in wm.monitors.iter().enumerate() {
            let screen_config = config_overlay.screens.get(i).unwrap_or(&config_overlay.screens[0]);
            let layout = layout::compute(screen_config, ctx.monitor.width, ctx.monitor.height, config_overlay.general.font_size as f64);
            if let Ok(renderer) = Renderer::new(ctx.monitor.width, ctx.monitor.height, i, layout, &config_overlay) {
                renderers.push(renderer);
            }
        }
        
        // Setup Tick Thread
        let (tick_thread_tx, tick_thread_rx) = bounded(1);
        let interval_rx_tick = interval_rx.clone();
        let initial_interval = Duration::from_millis(config_overlay.general.update_ms);
        thread::spawn(move || {
            let mut interval = initial_interval;
            loop {
                let start = Instant::now();
                if tick_thread_tx.send(()).is_err() { break; }
                while let Ok(new_interval) = interval_rx_tick.try_recv() {
                    interval = new_interval;
                }
                let elapsed = start.elapsed();
                if elapsed < interval { thread::sleep(interval - elapsed); }
                else { thread::sleep(Duration::from_millis(1)); }
            }
        });

        let keycode_w = find_keycode(&conn_arc, 0x0077).unwrap_or(Some(0)).unwrap_or(0);
        let keycode_q = find_keycode(&conn_arc, 0x0071).unwrap_or(Some(0)).unwrap_or(0);
        let mut visible = true;

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
                                                let _ = renderer.draw(&conn_arc, ev.window(), &config_overlay, &shared.data);
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
                        if let Ok(shared) = metrics_arc.lock() {
                            for (i, renderer) in renderers.iter_mut().enumerate() {
                                if let Some(ctx) = wm.monitors.get(i) {
                                    let _ = renderer.draw(&conn_arc, ctx.window, &config_overlay, &shared.data);
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
                            let _ = Command::new("notify-send").args(&["-t", "1000", "Matrix Overlay", "Reloading Configuration..."]).spawn();
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
                                let _ = Command::new("notify-send").args(&["-t", "1000", "Matrix Overlay", "Changes Applied Successfully"]).spawn();
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
        log::info!("Overlay logic thread stopping. Cleaning up windows...");
        let _ = wm.cleanup(&conn_arc);
    });

    // 7c. Spawn Productivity Thread (Auto-Commits & AI Insights)
    let productivity_config = config.clone();
    let productivity_shutdown = shutdown.clone();
    thread::spawn(move || {
        log::info!("Productivity thread started.");
        let mut last_commit_check = Instant::now();
        
        while !productivity_shutdown.load(Ordering::Relaxed) {
            // Run commit check every hour
            if last_commit_check.elapsed() >= Duration::from_secs(3600) {
                last_commit_check = Instant::now();
                if let Err(e) = run_auto_commit_cycle(&productivity_config) {
                    log::error!("Auto-commit cycle failed: {}", e);
                }
            }
            
            thread::sleep(Duration::from_secs(60));
        }
        log::info!("Productivity thread stopped.");
    });

    // Start GTK Main Loop on main thread
    #[cfg(target_os = "linux")]
    {
        log::info!("GTK dedicated thread active (60 FPS GUI).");
        loop {
            if shutdown.load(Ordering::Relaxed) { break; }
            while gtk::events_pending() {
                gtk::main_iteration();
            }
            
            // Watch for GUI events that need to be handled on the main thread (like opening a window)
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

            thread::sleep(Duration::from_millis(16)); // ~60 FPS responsiveness for UI
        }
    }

    log::info!("Shutting down main...");
    
    // Ungrab key (Optional as thread does it, but safer here if thread crashes)
    let keycode_w = find_keycode(&conn, 0x0077)?.unwrap_or(0);
    let keycode_q = find_keycode(&conn, 0x0071)?.unwrap_or(0);
    let _ = conn.send_request(&x::UngrabKey { key: keycode_w, grab_window: root, modifiers: x::ModMask::ANY });
    let _ = conn.send_request(&x::UngrabKey { key: keycode_q, grab_window: root, modifiers: x::ModMask::ANY });
    let _ = conn.flush();

    shutdown.store(true, Ordering::Relaxed);
    
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
        let cookie = conn.send_request_checked(&x::GrabKey {
            owner_events: true,
            grab_window: root,
            modifiers: mods,
            key: keycode,
            pointer_mode: x::GrabMode::Async,
            keyboard_mode: x::GrabMode::Async,
        });
        if let Err(e) = conn.check_request(cookie) {
            log::warn!("Failed to grab hotkey (keycode {}, mod {:?}): {}", keycode, mods, e);
        }
    }
    Ok(())
}

fn run_auto_commit_cycle(config: &Config) -> Result<()> {
    log::info!("Starting auto-commit cycle for {} repos...", config.productivity.repos.len());
    
    for repo_path in &config.productivity.repos {
        let path = Path::new(repo_path);
        if !path_utils::is_safe_path(path) {
            log::warn!("Skipping unsafe repo path: {}", repo_path);
            continue;
        }

        match Repository::open(path) {
            Ok(repo) => {
                if let Err(e) = handle_repo_auto_commit(&repo, config) {
                    log::error!("Failed to auto-commit in {}: {}", repo_path, e);
                }
            }
            Err(e) => log::warn!("Could not open repo at {}: {}", repo_path, e),
        }
    }
    
    Ok(())
}

fn handle_repo_auto_commit(repo: &Repository, config: &Config) -> Result<()> {
    let mut index = repo.index()?;
    let statuses = repo.statuses(None)?;
    
    if statuses.is_empty() {
        return Ok(());
    }

    // Check line count threshold
    let mut total_diff_lines = 0;
    if let Ok(diff) = repo.diff_index_to_workdir(None, None) {
        if let Ok(stats) = diff.stats() {
            total_diff_lines = stats.insertions() + stats.deletions();
        }
    }

    if total_diff_lines < config.productivity.auto_commit_threshold as usize {
        log::debug!("Skipping auto-commit: {} lines < {} threshold", total_diff_lines, config.productivity.auto_commit_threshold);
        return Ok(());
    }

    // Stage all changes
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent_commit = repo.head()?.peel_to_commit()?;
    let sig = repo.signature()?;

    let message = if config.productivity.ollama_enabled {
        generate_ai_commit_message(repo).unwrap_or_else(|_| "Auto-commit (Matrix Overlay)".to_string())
    } else {
        "Auto-commit (Matrix Overlay)".to_string()
    };

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&parent_commit])?;
    log::info!("Auto-committed to {}: {}", repo.path().display(), message);

    Ok(())
}

fn generate_ai_commit_message(repo: &Repository) -> Result<String> {
    // Basic diff for Ollama
    let diff = repo.diff_index_to_workdir(None, None)?;
    let mut diff_text = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        diff_text.extend_from_slice(line.content());
        true
    })?;

    let diff_str = String::from_utf8_lossy(&diff_text);
    let truncated_diff = if diff_str.len() > 4000 {
        format!("{}... [truncated]", &diff_str[..4000])
    } else {
        diff_str.to_string()
    };

    let prompt = format!(
        "Generate a concise one-line git commit message for the following diff:\n\n{}",
        truncated_diff
    );

    // Use reqwest blocking to call Ollama
    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({
        "model": "qwen2.5-coder:7b-instruct-q5_K_M",
        "prompt": prompt,
        "stream": false
    });

    let res = client.post("http://localhost:11434/api/generate")
        .json(&body)
        .send()?
        .json::<serde_json::Value>()?;

    if let Some(msg) = res["response"].as_str() {
        Ok(msg.trim().trim_matches('"').to_string())
    } else {
        bail!("Failed to get message from Ollama")
    }
}