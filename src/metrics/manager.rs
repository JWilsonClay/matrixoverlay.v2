//! Metrics Orchestration & Lifecycle Manager.
use chrono::Datelike;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use crossbeam_channel::{bounded, Sender, Receiver};
use sysinfo::{System, SystemExt, CpuExt};
use crate::core::config::Config;
use crate::metrics::*;

#[derive(Debug)]
pub struct SysinfoManager { pub system: System }

impl SysinfoManager {
    pub fn new() -> Self {
        let mut s = System::new_all(); s.refresh_all();
        Self { system: s }
    }
}

/// [HARDENED] Orchestrates metrics collection thread with resource governance.
pub fn spawn_metrics_thread(config: &Config) -> (Arc<Mutex<SharedMetrics>>, Arc<AtomicBool>, thread::JoinHandle<()>, Sender<MetricsCommand>) {
    let (tx, rx) = bounded(16); // [HARDENING] Bounded command channel
    let shared = Arc::new(Mutex::new(SharedMetrics::new()));
    let shutdown = Arc::new(AtomicBool::new(false));
    
    let s_clone = shared.clone();
    let d_clone = shutdown.clone();
    let cfg_init = config.clone();

    let handle = thread::spawn(move || {
        let sys = Arc::new(Mutex::new(SysinfoManager::new()));
        let mut cur_cfg = cfg_init;
        let mut colls = crate::metrics::dispatch::init_collectors(&cur_cfg, sys.clone());

        log::info!("Metrics Substrate: Initialized with {} collectors.", colls.len());

        while !d_clone.load(Ordering::SeqCst) { // [HARDENING] SeqCst shutdown
            let start = Instant::now();
            
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    MetricsCommand::UpdateConfig(n) => {
                        cur_cfg = n;
                        colls = crate::metrics::dispatch::init_collectors(&cur_cfg, sys.clone());
                    }
                    MetricsCommand::ForceRefresh => { log::debug!("Metrics: Force refresh."); }
                }
            }

            let mut frame = std::collections::HashMap::new();
            for c in &mut colls { frame.extend(c.collect()); }

            if let Ok(mut sh) = s_clone.lock() {
                sh.data = MetricData { values: frame };
                sh.timestamp = Instant::now();
                sh.day_of_week = chrono::Local::now().weekday().to_string();
            }

            let interval = Duration::from_millis(cur_cfg.general.update_ms.max(10));
            let elapsed = start.elapsed();
            if elapsed < interval { thread::sleep(interval - elapsed); }
        }
        log::info!("Metrics Substrate: Shutdown complete.");
    });

    (shared, shutdown, handle, tx)
}
