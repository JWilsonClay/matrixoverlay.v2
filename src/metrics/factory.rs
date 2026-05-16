// src/metrics/factory.rs
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use crate::core::config::Config;
use crate::metrics::{
    MetricId, MetricCollector, SysinfoManager, CpuCollector, MemoryCollector, 
    UptimeLoadCollector, NetworkCollector, DiskCollector, HwmonCollector, 
    NvidiaSmiCollector, OpenMeteoCollector, DateCollector, OverlayCpuCollector
};

pub fn create_collectors(config: &Config) -> Vec<Box<dyn MetricCollector>> {
    let sys = Arc::new(Mutex::new(SysinfoManager::new()));
    let mut ids = HashSet::new();
    
    // Default system metrics
    let defaults = [
        MetricId::CpuUsage, MetricId::RamUsage, MetricId::Uptime, 
        MetricId::NetworkDetails, MetricId::CpuTemp, MetricId::FanSpeed, MetricId::DayOfWeek
    ];
    for id in &defaults { ids.insert(id.clone()); }

    for screen in &config.screens {
        for m in &screen.metrics { if let Some(id) = MetricId::from_str(m) { ids.insert(id); } }
    }

    let mut collectors: Vec<Box<dyn MetricCollector>> = Vec::new();
    if ids.contains(&MetricId::CpuUsage) || ids.contains(&MetricId::LoadAvg) {
        collectors.push(Box::new(CpuCollector::new(sys.clone())));
    }
    if ids.contains(&MetricId::RamUsage) || ids.contains(&MetricId::RamUsed) {
        collectors.push(Box::new(MemoryCollector::new(sys.clone())));
    }
    if ids.contains(&MetricId::Uptime) { collectors.push(Box::new(UptimeLoadCollector::new(sys.clone()))); }
    if ids.contains(&MetricId::NetworkDetails) { collectors.push(Box::new(NetworkCollector::new())); }
    if ids.contains(&MetricId::DiskUsage) { collectors.push(Box::new(DiskCollector::new(sys.clone()))); }
    
    let unit = config.general.temp_unit.clone();
    if ids.contains(&MetricId::CpuTemp) || ids.contains(&MetricId::FanSpeed) {
        collectors.push(Box::new(HwmonCollector::new(unit.clone())));
    }
    if ids.contains(&MetricId::GpuTemp) || ids.contains(&MetricId::GpuUtil) {
        collectors.push(Box::new(NvidiaSmiCollector::new(unit)));
    }
    if config.weather.enabled {
        collectors.push(Box::new(OpenMeteoCollector::new(
            config.weather.lat, config.weather.lon, true, 
            config.weather.auto_location, config.general.temp_unit.clone()
        )));
    }
    if config.general.show_cpu_metric || ids.contains(&MetricId::OverlayCpu) {
        collectors.push(Box::new(OverlayCpuCollector::new(sys.clone())));
    }
    collectors.push(Box::new(DateCollector));
    collectors
}
