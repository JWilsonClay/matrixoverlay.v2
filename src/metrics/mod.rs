//! System metrics collection.
//! Uses sysinfo and other collectors to gather CPU, RAM, and GPU statistics.

pub mod collectors;
pub mod manager;
pub mod dispatch;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::fmt::Debug;
use crate::core::config::Config;

// Re-export collectors for ease of use
pub use self::collectors::system::*;
pub use self::collectors::hwmon::*;
pub use self::collectors::nvidia::*;
pub use self::collectors::git::*;
pub use self::collectors::file::*;
pub use self::collectors::weather::*;
pub use self::collectors::date::*;
pub use self::collectors::ai::*;

// Re-export manager functions
pub use self::manager::{spawn_metrics_thread, ResourceGuard, SysinfoManager};

#[derive(Debug, Clone)]
pub enum MetricsCommand {
    UpdateConfig(Config),
    ForceRefresh,
}

/// Unique identifier for metrics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MetricId {
    CpuUsage, RamUsage, RamUsed, RamTotal, LoadAvg, Uptime, NetworkDetails, DiskUsage, CpuTemp, FanSpeed, GpuTemp, GpuUtil, WeatherTemp, WeatherCondition, DayOfWeek, CodeDelta, OverlayCpu, LocationData, Custom(String),
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
            "location_data" => Some(Self::LocationData),
            other => Some(Self::Custom(other.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::CpuUsage => "cpu_usage", Self::RamUsage => "ram_usage", Self::RamUsed => "ram_used", Self::RamTotal => "ram_total", Self::LoadAvg => "load_avg", Self::Uptime => "uptime", Self::NetworkDetails => "network_details", Self::DiskUsage => "disk_usage", Self::CpuTemp => "cpu_temp", Self::FanSpeed => "fan_speed", Self::GpuTemp => "gpu_temp", Self::GpuUtil => "gpu_util", Self::WeatherTemp => "weather_temp", Self::WeatherCondition => "weather_condition", Self::DayOfWeek => "day_of_week", Self::CodeDelta => "code_delta", Self::OverlayCpu => "overlay_cpu", Self::LocationData => "location_data", Self::Custom(s) => s.as_str(),
        }
    }

    pub fn label(&self) -> String {
        self.as_str().replace("_", " ").to_uppercase()
    }
}

#[derive(Debug, Clone)]
pub struct MetricData {
    pub values: HashMap<MetricId, MetricValue>,
}

impl MetricData {
    pub fn summary(&self) -> String {
        format!("{} metrics active", self.values.len())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    Float(f64), Int(i64), String(String), NetworkMap(HashMap<String, (u64, u64)>), Location(f64, f64), None,
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

pub trait MetricCollector: Send + Sync + Debug {
    fn id(&self) -> &'static str;
    fn collect(&mut self) -> HashMap<MetricId, MetricValue>;
    fn label(&self) -> &'static str;
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
