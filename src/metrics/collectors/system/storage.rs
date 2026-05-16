// src/metrics/collectors/system/storage.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::{SystemExt, DiskExt};
use crate::metrics::{MetricId, MetricValue, MetricCollector, SysinfoManager};

#[derive(Debug)]
pub struct DiskCollector { sys: Arc<Mutex<SysinfoManager>> }

impl DiskCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self { Self { sys } }
}

impl MetricCollector for DiskCollector {
    fn id(&self) -> &'static str { "disk" }
    fn label(&self) -> &'static str { "Disk" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
            manager.system.refresh_disks();
            for disk in manager.system.disks() {
                if disk.mount_point() == std::path::Path::new("/") {
                    let (u, t) = (disk.total_space() - disk.available_space(), disk.total_space());
                    let p = if t > 0 { (u as f64 / t as f64) * 100.0 } else { 0.0 };
                    map.insert(MetricId::DiskUsage, MetricValue::String(format!("{:.1}%", p)));
                }
            }
        }
        map
    }
}

#[derive(Debug)]
pub struct UptimeLoadCollector { sys: Arc<Mutex<SysinfoManager>> }

impl UptimeLoadCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self { Self { sys } }
}

impl MetricCollector for UptimeLoadCollector {
    fn id(&self) -> &'static str { "uptime_load" }
    fn label(&self) -> &'static str { "System" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(manager) = self.sys.lock() {
            let u = manager.system.uptime();
            let (d, h, m) = (u / 86400, (u % 86400) / 3600, (u % 3600) / 60);
            let s = if d > 0 { format!("{} days {}:{:02}", d, h, m) } else { format!("{}:{:02}", h, m) };
            map.insert(MetricId::Uptime, MetricValue::String(s));
            map.insert(MetricId::LoadAvg, MetricValue::String(format!("{:.2}", manager.system.load_average().one)));
        }
        map
    }
}
