// src/metrics/collectors/system/memory.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::SystemExt;
use crate::metrics::{MetricId, MetricValue, MetricCollector, SysinfoManager};

#[derive(Debug)]
pub struct MemoryCollector { sys: Arc<Mutex<SysinfoManager>> }

impl MemoryCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self { Self { sys } }
}

impl MetricCollector for MemoryCollector {
    fn id(&self) -> &'static str { "memory" }
    fn label(&self) -> &'static str { "RAM" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
            manager.system.refresh_memory();
            let (used, total) = (manager.system.used_memory(), manager.system.total_memory());
            let used_gb = used as f64 / 1024.0 / 1024.0 / 1024.0;
            let percent = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
            map.insert(MetricId::RamUsed, MetricValue::String(format!("{:.1} GB", used_gb)));
            map.insert(MetricId::RamUsage, MetricValue::String(format!("{:.0}%", percent)));
        }
        map
    }
}
