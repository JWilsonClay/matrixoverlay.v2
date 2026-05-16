//! Timer and orchestration thread.
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::Sender;
use chrono::Datelike;
use crate::core::config::Config;
use crate::metrics::{SharedMetrics, MetricData, factory};

/// [HARDENED] Spawns metrics collection thread with rate-limiting and graceful shutdown.
pub fn spawn_metrics_and_timer_thread(
    config: &Config, metrics: Arc<Mutex<SharedMetrics>>, redraw_tx: Sender<()>, shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let config = config.clone();
    // [HARDENING] Enforce minimum interval to prevent CPU exhaustion
    let interval_ms = config.general.update_ms.max(10);

    thread::spawn(move || {
        let mut collectors = factory::create_collectors(&config);
        log::info!("Timer thread initialized. Interval: {}ms", interval_ms);
        let interval = Duration::from_millis(interval_ms);

        while !shutdown.load(Ordering::SeqCst) {
            let start = Instant::now();
            let mut frame_data = std::collections::HashMap::new();
            for collector in &mut collectors { frame_data.extend(collector.collect()); }

            if let Ok(mut shared) = metrics.lock() {
                shared.data = MetricData { values: frame_data };
                shared.timestamp = Instant::now();
                shared.day_of_week = chrono::Local::now().weekday().to_string();
            }

            if redraw_tx.send(()).is_err() { break; }

            let elapsed = start.elapsed();
            if elapsed < interval { thread::sleep(interval - elapsed); }
            else { thread::sleep(Duration::from_millis(1)); }
        }
        log::info!("Timer thread stopped cleanly.");
    })
}