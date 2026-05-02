use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs;
use std::time::Instant;
use sysinfo::{SystemExt, CpuExt, ProcessExt, DiskExt};
use crate::metrics::{MetricId, MetricValue, MetricCollector, SysinfoManager};

/// Collector for CPU usage (Total + Per Core).
#[derive(Debug)]
pub struct CpuCollector {
    sys: Arc<Mutex<SysinfoManager>>,
}

impl CpuCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { sys }
    }
}

impl MetricCollector for CpuCollector {
    fn id(&self) -> &'static str { "cpu" }
    fn label(&self) -> &'static str { "CPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        match self.sys.lock() {
            Ok(mut manager) => {
                manager.system.refresh_cpu();
                let global = manager.system.global_cpu_info().cpu_usage();
                map.insert(MetricId::CpuUsage, MetricValue::String(format!("{:.1}%", global)));
            },
            Err(e) => {
                log::error!("CpuCollector lock failed: {}", e);
                map.insert(MetricId::CpuUsage, MetricValue::String("ERR".to_string()));
            }
        }
        map
    }
}

/// Collector for Memory usage.
#[derive(Debug)]
pub struct MemoryCollector {
    sys: Arc<Mutex<SysinfoManager>>,
}

impl MemoryCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { sys }
    }
}

impl MetricCollector for MemoryCollector {
    fn id(&self) -> &'static str { "memory" }
    fn label(&self) -> &'static str { "RAM" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        match self.sys.lock() {
            Ok(mut manager) => {
                manager.system.refresh_memory();
                let used = manager.system.used_memory();
                let total = manager.system.total_memory();
                
                let used_gb = used as f64 / 1024.0 / 1024.0 / 1024.0;
                let percent = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                
                map.insert(MetricId::RamUsed, MetricValue::String(format!("{:.1} GB", used_gb)));
                map.insert(MetricId::RamUsage, MetricValue::String(format!("{:.0}%", percent)));
            },
            Err(e) => {
                log::error!("MemoryCollector lock failed: {}", e);
                map.insert(MetricId::RamUsage, MetricValue::String("ERR".to_string()));
            }
        }
        map
    }
}

/// Collector for Uptime and Load Average.
#[derive(Debug)]
pub struct UptimeLoadCollector {
    sys: Arc<Mutex<SysinfoManager>>,
}

impl UptimeLoadCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { sys }
    }
}

impl MetricCollector for UptimeLoadCollector {
    fn id(&self) -> &'static str { "uptime_load" }
    fn label(&self) -> &'static str { "System" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        match self.sys.lock() {
            Ok(manager) => {
                let uptime_secs = manager.system.uptime();
                let days = uptime_secs / 86400;
                let hours = (uptime_secs % 86400) / 3600;
                let mins = (uptime_secs % 3600) / 60;
                
                let uptime_str = if days > 0 {
                    format!("{} days {}:{:02}", days, hours, mins)
                } else {
                    format!("{}:{:02}", hours, mins)
                };
                
                map.insert(MetricId::Uptime, MetricValue::String(uptime_str));
                
                let load = manager.system.load_average();
                map.insert(MetricId::LoadAvg, MetricValue::String(format!("{:.2}", load.one)));
            },
            Err(e) => {
                log::error!("UptimeLoadCollector lock failed: {}", e);
                map.insert(MetricId::Uptime, MetricValue::String("ERR".to_string()));
            }
        }
        map
    }
}

/// Collector for Network usage (Bytes/sec).
#[derive(Debug)]
pub struct NetworkCollector {
    last_snapshot: HashMap<String, (u64, u64)>,
    last_collection_time: Instant,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            last_snapshot: HashMap::new(),
            last_collection_time: Instant::now(),
        }
    }

    fn read_proc_net_dev(&self) -> HashMap<String, (u64, u64)> {
        let mut map = HashMap::new();
        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                let line = line.trim();
                if let Some(colon_idx) = line.find(':') {
                    let iface = &line[..colon_idx];
                    let stats_str = &line[colon_idx+1..];
                    let stats: Vec<&str> = stats_str.split_whitespace().collect();
                    if stats.len() >= 9 {
                        if let (Ok(rx), Ok(tx)) = (stats[0].parse::<u64>(), stats[8].parse::<u64>()) {
                            map.insert(iface.to_string(), (rx, tx));
                        }
                    }
                }
            }
        }
        map
    }
    #[allow(dead_code)]
    fn format_rate(bytes_sec: f64) -> String {
        if bytes_sec >= 1_073_741_824.0 {
            format!("{:.1} GB/s", bytes_sec / 1_073_741_824.0)
        } else if bytes_sec >= 1_048_576.0 {
            format!("{:.1} MB/s", bytes_sec / 1_048_576.0)
        } else if bytes_sec >= 1024.0 {
            format!("{:.1} KB/s", bytes_sec / 1024.0)
        } else {
            format!("{:.0} B/s", bytes_sec)
        }
    }
}

impl MetricCollector for NetworkCollector {
    fn id(&self) -> &'static str { "network" }
    fn label(&self) -> &'static str { "Net" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let now = Instant::now();
        let current_snapshot = self.read_proc_net_dev();
        let _duration = now.duration_since(self.last_collection_time).as_secs_f64();
        let _duration = if _duration < 0.001 { 1.0 } else { _duration };

        let mut results = HashMap::new();
        let mut details_map = HashMap::new();

        for (iface, (curr_rx, curr_tx)) in &current_snapshot {
            if iface == "lo" { continue; }
            if let Some((last_rx, last_tx)) = self.last_snapshot.get(iface) {
                let delta_rx = if *curr_rx >= *last_rx { curr_rx - last_rx } else { 0 };
                let delta_tx = if *curr_tx >= *last_tx { curr_tx - last_tx } else { 0 };
                details_map.insert(iface.clone(), (delta_rx, delta_tx));
            }
        }

        results.insert(MetricId::NetworkDetails, MetricValue::NetworkMap(details_map));
        self.last_snapshot = current_snapshot;
        self.last_collection_time = now;

        results
    }
}

/// Collector for Disk usage.
#[derive(Debug)]
pub struct DiskCollector {
    sys: Arc<Mutex<SysinfoManager>>,
}

impl DiskCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { sys }
    }
}

impl MetricCollector for DiskCollector {
    fn id(&self) -> &'static str { "disk" }
    fn label(&self) -> &'static str { "Disk" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
            manager.system.refresh_disks_list();
            manager.system.refresh_disks();
            for disk in manager.system.disks() {
                if disk.mount_point() == std::path::Path::new("/") {
                     let used = disk.total_space() - disk.available_space();
                     let total = disk.total_space();
                     let percent = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                     map.insert(MetricId::DiskUsage, MetricValue::String(format!("{:.1}%", percent)));
                }
            }
        }
        map
    }
}

/// Collector for the overlay's own CPU usage.
#[derive(Debug)]
pub struct OverlayCpuCollector {
    sys: Arc<Mutex<SysinfoManager>>,
    pid: sysinfo::Pid,
}

impl OverlayCpuCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        use sysinfo::get_current_pid;
        let pid = get_current_pid().unwrap_or(sysinfo::Pid::from(0));
        Self { sys, pid }
    }
}

impl MetricCollector for OverlayCpuCollector {
    fn id(&self) -> &'static str { "overlay_cpu" }
    fn label(&self) -> &'static str { "Overlay CPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        match self.sys.lock() {
            Ok(mut manager) => {
                manager.system.refresh_process(self.pid);
                if let Some(process) = manager.system.process(self.pid) {
                    let cpu = process.cpu_usage();
                    let core_count = manager.system.cpus().len() as f32;
                    let normalized_cpu = if core_count > 0.0 { cpu / core_count } else { cpu };
                    
                    map.insert(MetricId::OverlayCpu, MetricValue::String(format!("{:.2}%", normalized_cpu)));
                }
            },
            Err(e) => {
                log::error!("OverlayCpuCollector lock failed: {}", e);
                map.insert(MetricId::OverlayCpu, MetricValue::String("ERR".to_string()));
            }
        }
        map
    }
}

/// Compatibility for tests
#[derive(Debug)]
pub struct SysinfoCollector {
    metric_id: MetricId,
    sys: Arc<Mutex<SysinfoManager>>,
}

impl SysinfoCollector {
    pub fn new(metric_id: MetricId, sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { metric_id, sys }
    }
}

impl MetricCollector for SysinfoCollector {
    fn id(&self) -> &'static str { "sysinfo_compat" }
    fn label(&self) -> &'static str { "SysinfoCompat" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
             match self.metric_id {
                MetricId::CpuUsage => {
                    manager.system.refresh_cpu();
                    let val = manager.system.global_cpu_info().cpu_usage();
                    map.insert(MetricId::CpuUsage, MetricValue::Float(val as f64));
                },
                MetricId::RamUsage => {
                    manager.system.refresh_memory();
                    let used = manager.system.used_memory();
                    let total = manager.system.total_memory();
                    let val = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                    map.insert(MetricId::RamUsage, MetricValue::Float(val));
                },
                MetricId::Uptime => {
                    let val = manager.system.uptime();
                    map.insert(MetricId::Uptime, MetricValue::Int(val as i64));
                },
                _ => {}
             }
        }
        map
    }
}
