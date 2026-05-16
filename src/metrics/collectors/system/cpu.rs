// src/metrics/collectors/system/cpu.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::{SystemExt, CpuExt};
use crate::metrics::{MetricId, MetricValue, MetricCollector, SysinfoManager};

#[derive(Debug)]
pub struct CpuCollector { sys: Arc<Mutex<SysinfoManager>> }

impl CpuCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self { Self { sys } }
}

impl MetricCollector for CpuCollector {
    fn id(&self) -> &'static str { "cpu" }
    fn label(&self) -> &'static str { "CPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
            manager.system.refresh_cpu();
            let global = manager.system.global_cpu_info().cpu_usage();
            map.insert(MetricId::CpuUsage, MetricValue::String(format!("{:.1}%", global)));
        }
        map
    }
}
