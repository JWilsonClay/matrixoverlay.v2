//! Timer and orchestration thread.
//! Handles the main update loop: collecting metrics and signaling the main thread to redraw.

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use chrono::Datelike;

use crate::config::Config;
use crate::metrics::{
    SharedMetrics, MetricData, MetricId, MetricCollector,
    SysinfoManager, CpuCollector, MemoryCollector, UptimeLoadCollector,
    NetworkCollector, DiskCollector, HwmonCollector, NvidiaSmiCollector,
    OpenMeteoCollector, DateCollector
};

/// Spawns a thread that collects metrics and signals a redraw event at a fixed interval.
///
/// This replaces the internal loop of `metrics::spawn_metrics_thread` with one that
/// explicitly communicates with the main thread via `redraw_tx`.
pub fn spawn_metrics_and_timer_thread(
    config: &Config,
    metrics: Arc<Mutex<SharedMetrics>>,
    redraw_tx: Sender<()>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let config = config.clone();
    let interval_ms = config.general.update_ms;

    thread::spawn(move || {
        let sys_manager = Arc::new(Mutex::new(SysinfoManager::new()));
        let mut collectors: Vec<Box<dyn MetricCollector>> = Vec::new();

        // 1. Identify required metrics from config
        let mut required_metrics = HashSet::new();
        
        // Always add shared/base metrics
        required_metrics.insert(MetricId::CpuUsage);
        required_metrics.insert(MetricId::RamUsage);
        required_metrics.insert(MetricId::Uptime);
        required_metrics.insert(MetricId::NetworkDetails);
        required_metrics.insert(MetricId::CpuTemp);
        required_metrics.insert(MetricId::FanSpeed);
        required_metrics.insert(MetricId::DayOfWeek);

        // Add per-screen unique metrics
        for screen in &config.screens {
            for metric_name in &screen.metrics {
                if let Some(id) = MetricId::from_str(metric_name) {
                    required_metrics.insert(id);
                }
            }
        }

        // 2. Register Collectors based on requirements
        if required_metrics.contains(&MetricId::CpuUsage) || required_metrics.contains(&MetricId::LoadAvg) {
            collectors.push(Box::new(CpuCollector::new(sys_manager.clone())));
        }
        if required_metrics.contains(&MetricId::RamUsage) || required_metrics.contains(&MetricId::RamUsed) || required_metrics.contains(&MetricId::RamTotal) {
            collectors.push(Box::new(MemoryCollector::new(sys_manager.clone())));
        }
        if required_metrics.contains(&MetricId::Uptime) || required_metrics.contains(&MetricId::LoadAvg) {
            collectors.push(Box::new(UptimeLoadCollector::new(sys_manager.clone())));
        }
        if required_metrics.contains(&MetricId::NetworkDetails) {
            collectors.push(Box::new(NetworkCollector::new()));
        }
        if required_metrics.contains(&MetricId::DiskUsage) {
            collectors.push(Box::new(DiskCollector::new(sys_manager.clone())));
        }
        if required_metrics.contains(&MetricId::CpuTemp) || required_metrics.contains(&MetricId::FanSpeed) || required_metrics.contains(&MetricId::GpuTemp) {
            collectors.push(Box::new(HwmonCollector::new()));
        }
        if required_metrics.contains(&MetricId::GpuTemp) || required_metrics.contains(&MetricId::GpuUtil) {
             collectors.push(Box::new(NvidiaSmiCollector::new()));
        }
        if config.weather.enabled {
            collectors.push(Box::new(OpenMeteoCollector::new(config.weather.lat, config.weather.lon, true)));
        }
        collectors.push(Box::new(DateCollector));

        log::info!("Timer thread initialized with {} collectors. Interval: {}ms", collectors.len(), interval_ms);

        let interval = Duration::from_millis(interval_ms);

        while !shutdown.load(Ordering::Relaxed) {
            let start_time = Instant::now();
            
            // Collect
            let mut frame_data = HashMap::new();
            for collector in &mut collectors {
                let data = collector.collect();
                frame_data.extend(data);
            }

            // Update Shared State
            if let Ok(mut shared) = metrics.lock() {
                shared.data = MetricData { values: frame_data };
                shared.timestamp = Instant::now();
                shared.day_of_week = chrono::Local::now().weekday().to_string();

                if log::log_enabled!(log::Level::Debug) {
                    log::debug!("Metrics Collected: {}", shared.data.summary());
                }
            }

            // Signal Redraw
            if let Err(_) = redraw_tx.send(()) {
                log::info!("Redraw channel closed, stopping timer thread.");
                break;
            }

            // Sleep remainder of interval
            let elapsed = start_time.elapsed();
            if elapsed < interval {
                thread::sleep(interval - elapsed);
            }
        }
        log::info!("Timer thread stopped.");
    })
}