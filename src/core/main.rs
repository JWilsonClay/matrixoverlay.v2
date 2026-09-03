// src/core/main.rs
use anyhow::{Context, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::thread;
use crossbeam_channel::unbounded;

use crate::core::config::Config;
use crate::core::update::{UpdateManager, UpdateEvent};
use crate::core::{init, threads};
use crate::ui::gui::{GuiEvent, ConfigWindow};

/// Set by the SIGTERM/SIGINT handler so the main loop can exit through the normal
/// shutdown path. Required by task 1.7: the S-13b summary is printed at the end of
/// `run()`, so a process terminated externally would otherwise never emit it — the
/// measurement would be unreachable in exactly the case where it is taken.
static TERMINATE: AtomicBool = AtomicBool::new(false);

/// Async-signal-safe: an atomic store is the only thing this does.
extern "C" fn handle_terminate(_sig: libc::c_int) {
    TERMINATE.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    // Safety: `handle_terminate` performs only an atomic store, which is
    // async-signal-safe. Registering it cannot fail in a way we can act on.
    unsafe {
        libc::signal(libc::SIGTERM, handle_terminate as libc::sighandler_t);
        libc::signal(libc::SIGINT, handle_terminate as libc::sighandler_t);
    }
}

pub fn run() -> Result<()> {
    // 1. Load Config
    let mut config = Config::load().context("Failed to load configuration")?;
    
    // 2. Init Substrate
    init::init_logging(&config)?;
    gtk::init().context("Failed to initialize GTK")?;
    install_signal_handlers();
    log::info!("Sovereign Substrate Initialized. v0.1.3-SOC");

    // [HARDENING] Verify DISPLAY for X11 environments
    if std::env::var("DISPLAY").is_err() {
        anyhow::bail!("X11 Error: DISPLAY environment variable is not set. Matrix Overlay requires an active X session.");
    }

    config.cosmetics.rain_mode = "fall".to_string();
    
    // 3. Spawn Metrics Hub
    let (metrics, shutdown, _metrics_handle, metrics_tx) = crate::metrics::spawn_metrics_thread(&config);

    // 4. Setup XCB Connection
    let (conn, _screen_num) = init::setup_xcb()?;

    // 5. Initial HUD Environment
    if let Err(e) = init::safe_exec("xsetroot", &["-solid", "#000000"]) {
        log::warn!("Failed to set background: {}", e);
    }

    if let Err(e) = init::setup_autostart() {
        log::warn!("Failed to setup autostart: {}", e);
    }

    // 6. Channel Architecture
    let (xcb_tx, xcb_rx) = unbounded::<xcb::Event>();
    let (gui_tx, gui_rx) = unbounded::<GuiEvent>();
    let (control_tx, control_rx) = unbounded::<GuiEvent>();
    let (update_tx, update_rx) = unbounded::<UpdateEvent>();

    // 7. Initialize Update Manager
    let update_manager = Arc::new(UpdateManager::new("JWilsonClay", "matrixoverlay.v2", update_tx));
    let _update_handle = (*update_manager).clone().spawn_checker(24, Arc::clone(&shutdown));
    
    // 8. Spawn Core Threads
    let _tray = crate::ui::tray::SystemTray::new(&config)?;
    
    threads::spawn_xcb_thread(Arc::clone(&conn), xcb_tx);
    threads::spawn_productivity_thread(config.clone(), Arc::clone(&shutdown));
    
    threads::spawn_overlay_thread(
        Arc::clone(&conn),
        config.clone(),
        Arc::clone(&shutdown),
        Arc::clone(&metrics),
        metrics_tx,
        xcb_rx,
        gui_rx,
        control_tx.clone(),
        update_rx,
        update_manager,
    );

    // 9. Main GUI Loop (GTK)
    #[cfg(target_os = "linux")]
    {
        while !shutdown.load(Ordering::Relaxed) && !TERMINATE.load(Ordering::Relaxed) {
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

    // [HARDENING] Graceful shutdown signal
    shutdown.store(true, Ordering::SeqCst);

    // [S-13b] Print the per-CRTC present-path summary exactly once, here at exit.
    // Deliberately not logged per present: a log line on the path being measured
    // is a new cost centre inside the measurement (implementation plan, task 1.7).
    print!("{}", crate::core::telemetry::summary());

    log::info!("Sovereign Substrate Shutting Down Cleanly.");
    Ok(())
}