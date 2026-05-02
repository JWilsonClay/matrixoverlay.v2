//! System metrics collection.
//! Uses sysinfo and other collectors to gather CPU, RAM, and GPU statistics.

pub mod collectors;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use std::fmt::Debug;
use chrono::Datelike;
use crate::core::config::Config;
use sysinfo::{System, SystemExt, CpuExt};
use crossbeam_channel::{unbounded, Sender};

// Re-export collectors for ease of use
pub use self::collectors::system::*;
pub use self::collectors::hwmon::*;
pub use self::collectors::nvidia::*;
pub use self::collectors::git::*;
pub use self::collectors::file::*;
pub use self::collectors::weather::*;
pub use self::collectors::date::*;
pub use self::collectors::ai::*;

#[derive(Debug, Clone)]
pub enum MetricsCommand {
    UpdateConfig(Config),
    ForceRefresh,
}

/// Unique identifier for metrics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MetricId {
    CpuUsage,
    RamUsage,
    RamUsed,
    RamTotal,
    LoadAvg,
    Uptime,
    NetworkDetails,
    DiskUsage,
    CpuTemp,
    FanSpeed,
    GpuTemp,
    GpuUtil,
    WeatherTemp,
    WeatherCondition,
    DayOfWeek,
    CodeDelta,
    OverlayCpu,
    Custom(String),
}

impl MetricId {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cpu_usage" => Some(Self::CpuUsage),
            "ram_usage" => Some(Self::RamUsage),
            "ram_used" => Some(Self::RamUsed),
            "ram_total" => Some(Self::RamTotal),
            "load_avg" => Some(Self::LoadAvg),
            "uptime" => Some(Self::Uptime),
            "network_details" => Some(Self::NetworkDetails),
            "disk_usage" => Some(Self::DiskUsage),
            "cpu_temp" => Some(Self::CpuTemp),
            "fan_speed" => Some(Self::FanSpeed),
            "gpu_temp" => Some(Self::GpuTemp),
            "gpu_util" => Some(Self::GpuUtil),
            "weather_temp" => Some(Self::WeatherTemp),
            "weather_condition" => Some(Self::WeatherCondition),
            "day_of_week" => Some(Self::DayOfWeek),
            "code_delta" => Some(Self::CodeDelta),
            "overlay_cpu" => Some(Self::OverlayCpu),
            other => Some(Self::Custom(other.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::CpuUsage => "cpu_usage",
            Self::RamUsage => "ram_usage",
            Self::RamUsed => "ram_used",
            Self::RamTotal => "ram_total",
            Self::LoadAvg => "load_avg",
            Self::Uptime => "uptime",
            Self::NetworkDetails => "network_details",
            Self::DiskUsage => "disk_usage",
            Self::CpuTemp => "cpu_temp",
            Self::FanSpeed => "fan_speed",
            Self::GpuTemp => "gpu_temp",
            Self::GpuUtil => "gpu_util",
            Self::WeatherTemp => "weather_temp",
            Self::WeatherCondition => "weather_condition",
            Self::DayOfWeek => "day_of_week",
            Self::CodeDelta => "code_delta",
            Self::OverlayCpu => "overlay_cpu",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::CpuUsage => "CPU",
            Self::RamUsage => "RAM %",
            Self::RamUsed => "RAM GB",
            Self::RamTotal => "RAM Max",
            Self::LoadAvg => "Load",
            Self::Uptime => "Uptime",
            Self::NetworkDetails => "Network",
            Self::DiskUsage => "Disk",
            Self::CpuTemp => "CPU Temp",
            Self::FanSpeed => "Fan",
            Self::GpuTemp => "GPU Temp",
            Self::GpuUtil => "GPU Util",
            Self::WeatherTemp => "Temp",
            Self::WeatherCondition => "Weather",
            Self::DayOfWeek => "Day",
            Self::CodeDelta => "Delta",
            Self::OverlayCpu => "Overlay CPU",
            Self::Custom(s) => s.as_str(),
        }.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct MetricData {
    pub values: HashMap<MetricId, MetricValue>,
}

impl MetricData {
    pub fn summary(&self) -> String {
        let count = self.values.len();
        let mut entries: Vec<_> = self.values.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());
        
        let sample: String = entries.iter().take(3).map(|(k, v)| {
            match v {
                MetricValue::NetworkMap(_) => format!("{:?}: <Map>", k),
                MetricValue::Float(f) => format!("{:?}: {:.1}", k, f),
                MetricValue::Int(i) => format!("{:?}: {}", k, i),
                MetricValue::String(s) => format!("{:?}: \"{}\"", k, s),
                MetricValue::None => format!("{:?}: None", k),
            }
        }).collect::<Vec<_>>().join(", ");
        
        format!("Count: {}, Sample: [{}{}]", count, sample, if count > 3 { ", ..." } else { "" })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    Float(f64),
    Int(i64),
    String(String),
    NetworkMap(HashMap<String, (u64, u64)>),
    None,
}

#[derive(Debug)]
pub struct SharedMetrics {
    pub data: MetricData,
    pub timestamp: Instant,
    pub day_of_week: String,
}

impl SharedMetrics {
    pub fn new() -> Self {
        Self {
            data: MetricData { values: HashMap::new() },
            timestamp: Instant::now(),
            day_of_week: "Unknown".to_string(),
        }
    }
}

/// Helper to monitor system load and throttle background operations.
#[derive(Debug, Clone)]
pub struct ResourceGuard {
    pub cpu_threshold: f32,
}

impl ResourceGuard {
    pub fn new(threshold: f32) -> Self {
        Self { cpu_threshold: threshold }
    }

    pub fn should_throttle(&self, sys_manager: &mut SysinfoManager) -> bool {
        sys_manager.system.refresh_cpu();
        sys_manager.system.global_cpu_info().cpu_usage() > self.cpu_threshold
    }
}

pub trait MetricCollector: Send + Sync + Debug {
    fn id(&self) -> &'static str;
    fn collect(&mut self) -> HashMap<MetricId, MetricValue>;
    fn label(&self) -> &'static str;
}

#[derive(Debug)]
pub struct MetricsManager {
    pub collectors: Vec<Box<dyn MetricCollector>>,
    pub shared: Arc<Mutex<SharedMetrics>>,
    pub shutdown: Arc<AtomicBool>,
    pub update_interval: u64,
}

/// Manages the sysinfo::System instance.
pub struct SysinfoManager {
    pub system: System,
}

impl SysinfoManager {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }
}

impl Debug for SysinfoManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SysinfoManager").finish()
    }
}

/// Spawns the metrics collection thread.
pub fn spawn_metrics_thread(config: &Config) -> (Arc<Mutex<SharedMetrics>>, Arc<AtomicBool>, thread::JoinHandle<()>, Sender<MetricsCommand>) {
    let (tx, rx) = unbounded();
    let shared_metrics = Arc::new(Mutex::new(SharedMetrics::new()));
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    
    let shared_clone = shared_metrics.clone();
    let shutdown_clone = shutdown_flag.clone();
    let config_initial = config.clone();

    let handle = thread::spawn(move || {
        let sys_manager = Arc::new(Mutex::new(SysinfoManager::new()));
        let mut current_config = config_initial;
        
        let mut collectors: Vec<Box<dyn MetricCollector>> = init_collectors(&current_config, sys_manager.clone());
        let guard = ResourceGuard::new(70.0);

        log::info!("Metrics thread initialized with {} collectors.", collectors.len());

        while !shutdown_clone.load(Ordering::Relaxed) {
            if let Ok(mut sys) = sys_manager.lock() {
                if guard.should_throttle(&mut sys) {
                    log::debug!("Metrics thread: Throttling due to high CPU load");
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }
            }

            let start_time = Instant::now();
            
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    MetricsCommand::UpdateConfig(new_cfg) => {
                        log::info!("Metrics thread: Reloading configuration...");
                        current_config = new_cfg;
                        collectors = init_collectors(&current_config, sys_manager.clone());
                    }
                    MetricsCommand::ForceRefresh => {
                        log::info!("Metrics thread: Force refresh requested.");
                    }
                }
            }

            let mut frame_data = HashMap::new();
            for collector in &mut collectors {
                let data = collector.collect();
                frame_data.extend(data);
            }

            if let Ok(mut shared) = shared_clone.lock() {
                shared.data = MetricData { values: frame_data };
                shared.timestamp = Instant::now();
                shared.day_of_week = chrono::Local::now().weekday().to_string();
            }

            let interval = Duration::from_millis(current_config.general.update_ms);
            let elapsed = start_time.elapsed();
            if elapsed < interval {
                thread::sleep(interval - elapsed);
            }
        }
        log::info!("Metrics thread stopped.");
    });

    (shared_metrics, shutdown_flag, handle, tx)
}

fn init_collectors(config: &Config, sys_manager: Arc<Mutex<SysinfoManager>>) -> Vec<Box<dyn MetricCollector>> {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_path_traversal_blocked() {
        assert!(!crate::core::path_utils::is_safe_path(Path::new("/etc/passwd")));
        assert!(!crate::core::path_utils::is_safe_path(Path::new("../.ssh/id_rsa")));
    }
}
