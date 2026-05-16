// src/metrics/manager.rs
use chrono::Datelike;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use crossbeam_channel::{unbounded, Sender, Receiver};
use sysinfo::{System, SystemExt, CpuExt};
use crate::core::config::Config;
use crate::metrics::*;

/// Manages the sysinfo::System instance.
#[derive(Debug)]
pub struct SysinfoManager {
    pub system: System,
}

impl SysinfoManager {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }
}

/// Helper to monitor system load and throttle background operations.
#[derive(Debug, Clone)]
pub struct ResourceGuard {
    pub cpu_threshold: f32,
}

impl ResourceGuard {
    pub fn new(threshold: f32) -> Self {
        Self { cpu_threshold: threshold }
    }

    pub fn should_throttle(&self, sys_manager: &mut SysinfoManager) -> bool {
        sys_manager.system.refresh_cpu();
        sys_manager.system.global_cpu_info().cpu_usage() > self.cpu_threshold
    }
}

pub fn spawn_metrics_thread(config: &Config) -> (Arc<Mutex<SharedMetrics>>, Arc<AtomicBool>, thread::JoinHandle<()>, Sender<MetricsCommand>) {
    let (tx, rx) = unbounded();
    let shared_metrics = Arc::new(Mutex::new(SharedMetrics::new()));
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    
    let shared_clone = shared_metrics.clone();
    let shutdown_clone = shutdown_flag.clone();
    let config_initial = config.clone();

    let handle = thread::spawn(move || {
        let sys_manager = Arc::new(Mutex::new(SysinfoManager::new()));
        let mut current_config = config_initial;
        
        let mut collectors: Vec<Box<dyn MetricCollector>> = crate::metrics::dispatch::init_collectors(&current_config, sys_manager.clone());
        let guard = ResourceGuard::new(70.0);

        log::info!("Metrics thread initialized with {} collectors.", collectors.len());

        while !shutdown_clone.load(Ordering::Relaxed) {
            if let Ok(mut sys) = sys_manager.lock() {
                if guard.should_throttle(&mut sys) {
                    log::debug!("Metrics thread: Throttling due to high CPU load");
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }
            }

            let start_time = Instant::now();
            
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    MetricsCommand::UpdateConfig(new_cfg) => {
                        log::info!("Metrics thread: Reloading configuration...");
                        current_config = new_cfg;
                        collectors = crate::metrics::dispatch::init_collectors(&current_config, sys_manager.clone());
                    }
                    MetricsCommand::ForceRefresh => {
                        log::info!("Metrics thread: Force refresh requested.");
                    }
                }
            }

            let mut frame_data = std::collections::HashMap::new();
            for collector in &mut collectors {
                let data = collector.collect();
                frame_data.extend(data);
            }

            if let Ok(mut shared) = shared_clone.lock() {
                shared.data = MetricData { values: frame_data };
                shared.timestamp = Instant::now();
                shared.day_of_week = chrono::Local::now().weekday().to_string();
            }

            let interval = Duration::from_millis(current_config.general.update_ms);
            let elapsed = start_time.elapsed();
            if elapsed < interval {
                thread::sleep(interval - elapsed);
            }
        }
        log::info!("Metrics thread stopped.");
    });

    (shared_metrics, shutdown_flag, handle, tx)
}
