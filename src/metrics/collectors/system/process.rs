// src/metrics/collectors/system/process.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::{SystemExt, ProcessExt, CpuExt, get_current_pid, Pid};
use crate::metrics::{MetricId, MetricValue, MetricCollector, SysinfoManager};

#[derive(Debug)]
pub struct OverlayCpuCollector {
    sys: Arc<Mutex<SysinfoManager>>,
    pid: Pid,
}

impl OverlayCpuCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { sys, pid: get_current_pid().unwrap_or(Pid::from(0)) }
    }
}

impl MetricCollector for OverlayCpuCollector {
    fn id(&self) -> &'static str { "overlay_cpu" }
    fn label(&self) -> &'static str { "Overlay CPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
            manager.system.refresh_process(self.pid);
            if let Some(p) = manager.system.process(self.pid) {
                let cpu = p.cpu_usage();
                let cores = manager.system.cpus().len() as f32;
                let norm = if cores > 0.0 { cpu / cores } else { cpu };
                map.insert(MetricId::OverlayCpu, MetricValue::String(format!("{:.2}%", norm)));
            }
        }
        map
    }
}
