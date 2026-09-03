//! Metrics Collector Dispatch Substrate.
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use crate::core::config::Config;
use crate::metrics::*;

/// [HARDENED] Initializes a subset of whitelisted collectors based on active configuration.
pub fn init_collectors(config: &Config, sys_manager: Arc<Mutex<SysinfoManager>>) -> Vec<Box<dyn MetricCollector>> {
    let mut collectors: Vec<Box<dyn MetricCollector>> = Vec::new();
    let mut required = HashSet::new();
    
    // Mandatory baseline metrics
    required.insert(MetricId::CpuUsage);
    required.insert(MetricId::RamUsage);
    required.insert(MetricId::Uptime);
    required.insert(MetricId::DayOfWeek);

    for screen in &config.screens {
        for m in &screen.metrics {
            if let Some(id) = MetricId::from_str(m) { required.insert(id); }
        }
    }

    if required.contains(&MetricId::CpuUsage) { collectors.push(Box::new(CpuCollector::new(sys_manager.clone()))); }
    if required.contains(&MetricId::RamUsage) { collectors.push(Box::new(MemoryCollector::new(sys_manager.clone()))); }
    if required.contains(&MetricId::Uptime) { collectors.push(Box::new(UptimeLoadCollector::new(sys_manager.clone()))); }
    if required.contains(&MetricId::NetworkDetails) { collectors.push(Box::new(NetworkCollector::new())); }
    if required.contains(&MetricId::DiskUsage) { collectors.push(Box::new(DiskCollector::new(sys_manager.clone()))); }
    
    if required.contains(&MetricId::CpuTemp) || required.contains(&MetricId::FanSpeed) {
        collectors.push(Box::new(HwmonCollector::new(config.general.temp_unit.clone())));
    }
    if required.contains(&MetricId::GpuTemp) || required.contains(&MetricId::GpuUtil) {
        collectors.push(Box::new(NvidiaSmiCollector::new(config.general.temp_unit.clone())));
    }
    if !config.productivity.repos.is_empty() {
        collectors.push(Box::new(GitCollector::new(config.productivity.repos.clone())));
    }
    if config.weather.enabled {
        collectors.push(Box::new(OpenMeteoCollector::new(
            config.weather.lat, config.weather.lon, true, config.weather.auto_location, config.general.temp_unit.clone()
        )));
    }
    if config.general.show_cpu_metric || required.contains(&MetricId::OverlayCpu) {
        collectors.push(Box::new(OverlayCpuCollector::new(sys_manager.clone())));
    }
    
    if required.contains(&MetricId::Fps) {
        collectors.push(Box::new(FpsCollector::new()));
    }

    collectors.push(Box::new(DateCollector));
    collectors
}
