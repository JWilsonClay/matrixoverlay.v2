// src/metrics/dispatch.rs
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use crate::core::config::Config;
use crate::metrics::*;
use crate::metrics::manager::SysinfoManager;

pub fn init_collectors(config: &Config, sys_manager: Arc<Mutex<SysinfoManager>>) -> Vec<Box<dyn MetricCollector>> {
    let mut collectors: Vec<Box<dyn MetricCollector>> = Vec::new();
    let mut required_metrics = HashSet::new();
    
    required_metrics.insert(MetricId::CpuUsage);
    required_metrics.insert(MetricId::RamUsage);
    required_metrics.insert(MetricId::Uptime);
    required_metrics.insert(MetricId::DayOfWeek);

    for screen in &config.screens {
        for m in &screen.metrics {
            if let Some(id) = MetricId::from_str(m) {
                required_metrics.insert(id);
            }
        }
    }

    if required_metrics.contains(&MetricId::CpuUsage) || required_metrics.contains(&MetricId::LoadAvg) {
        collectors.push(Box::new(CpuCollector::new(sys_manager.clone())));
    }
    if required_metrics.contains(&MetricId::RamUsage) || required_metrics.contains(&MetricId::RamUsed) {
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
    if required_metrics.contains(&MetricId::CpuTemp) || required_metrics.contains(&MetricId::FanSpeed) {
        collectors.push(Box::new(HwmonCollector::new(config.general.temp_unit.clone())));
    }
    if required_metrics.contains(&MetricId::GpuTemp) || required_metrics.contains(&MetricId::GpuUtil) {
        collectors.push(Box::new(NvidiaSmiCollector::new(config.general.temp_unit.clone())));
    }
    if !config.productivity.repos.is_empty() {
        collectors.push(Box::new(GitCollector::new(config.productivity.repos.clone())));
    }
    if config.weather.enabled {
        collectors.push(Box::new(OpenMeteoCollector::new_with_unit(config.weather.lat, config.weather.lon, true, config.weather.auto_location, config.general.temp_unit.clone())));
    }
    if config.general.show_cpu_metric || required_metrics.contains(&MetricId::OverlayCpu) {
        collectors.push(Box::new(OverlayCpuCollector::new(sys_manager.clone())));
    }
    
    collectors.push(Box::new(DateCollector));
    collectors
}
