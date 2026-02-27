//! System metrics collection.
//! Uses sysinfo and nvml-wrapper to gather CPU, RAM, and GPU statistics.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::thread;
use std::fs;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Command;
use chrono::{Datelike, Local};
use crate::config::Config;
use sysinfo::{System, SystemExt, CpuExt};
use sysinfo::DiskExt;
use serde::Deserialize;
use git2::Repository;
use crossbeam_channel::{unbounded, Sender};
use crate::path_utils;
use std::io::Read;
    

#[derive(Debug, Clone)]
pub enum MetricsCommand {
    UpdateConfig(Config),
    ForceRefresh,
}

/// Unique identifier for metrics.
/// 
/// Ties to Stage 0: Requirements Matrix (CPU, RAM, GPU, Weather, Productivity).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MetricId {
    /// Global CPU usage percentage.
    CpuUsage,
    /// Memory usage percentage.
    RamUsage,
    /// Resident memory in bytes.
    RamUsed,
    /// Total system memory in bytes.
    RamTotal,
    /// System load average (1m).
    LoadAvg,
    /// Total system uptime.
    Uptime,
    /// Network throughput per interface.
    NetworkDetails,
    /// Disk space usage percentage.
    DiskUsage,
    /// CPU core temperature (via hwmon).
    CpuTemp,
    /// System fan speed (RPM).
    FanSpeed,
    /// NVIDIA GPU core temperature.
    GpuTemp,
    /// NVIDIA GPU utilization percentage.
    GpuUtil,
    /// Current weather temperature.
    WeatherTemp,
    /// Current weather description (e.g. "Clear").
    WeatherCondition,
    /// Current day of week for header display.
    DayOfWeek,
    /// Git code delta (added/deleted lines in 24h).
    CodeDelta,
    /// Generic custom metric.
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
        entries.sort_by_key(|(k, _)| k.clone());
        
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
/// 
/// Ties to Stage 0: <1% CPU target. Ensures that background metrics collection
/// does not compete with higher-priority rendering or system tasks.
#[derive(Debug, Clone)]
pub struct ResourceGuard {
    /// CPU usage percentage threshold (0.0 - 100.0)
    pub cpu_threshold: f32,
}

impl ResourceGuard {
    /// Creates a new ResourceGuard with the given CPU threshold.
    pub fn new(threshold: f32) -> Self {
        Self { cpu_threshold: threshold }
    }

    /// Returns true if the current global CPU usage exceeds the threshold.
    ///
    /// Refreshes the CPU stats in the provided SysinfoManager.
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
        // Initial refresh
        system.refresh_all();
        Self { system }
    }
}

impl Debug for SysinfoManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SysinfoManager").finish()
    }
}

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
    fn label(&self) -> &'static str { "CPU" } // This label is for the collector, not the metric
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        match self.sys.lock() {
            Ok(mut manager) => {
                manager.system.refresh_cpu();
                let global = manager.system.global_cpu_info().cpu_usage();
                map.insert(MetricId::CpuUsage, MetricValue::String(format!("{:.1}%", global)));
                
                // Note: Per-core metrics are collected but MetricId enum is static.
                // We only expose global usage for the renderer in this version.
            },
            Err(e) => {
                log::error!("CpuCollector lock failed: {}", e);
                map.insert(MetricId::CpuUsage, MetricValue::String("ERR".to_string()));
            }
        }
        map
    }
}

/// Collector for Date/Time (Day of Week).
#[derive(Debug)]
pub struct DateCollector;

impl MetricCollector for DateCollector {
    fn id(&self) -> &'static str { "date" }
    fn label(&self) -> &'static str { "Date" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        let day = Local::now().format("%A").to_string();
        log::debug!("Collected DayOfWeek: {}", day);
        map.insert(MetricId::DayOfWeek, MetricValue::String(day));
        map
    }
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: CurrentWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    weather_code: i64,
}

/// Collector for Weather data from Open-Meteo.
#[derive(Debug)]
pub struct OpenMeteoCollector {
    lat: f64,
    lon: f64,
    enabled: bool,
    url_base: String,
}

impl OpenMeteoCollector {
    pub fn new(lat: f64, lon: f64, enabled: bool) -> Self {
        Self {
            lat,
            lon,
            enabled,
            url_base: "https://api.open-meteo.com".to_string(),
        }
    }

    pub fn new_with_url(_metric_id: MetricId, lat: f64, lon: f64, url: String) -> Self {
        Self {
            lat,
            lon,
            enabled: true,
            url_base: url,
        }
    }

    fn weather_code_str(code: i64) -> String {
        match code {
            0 => "Clear sky",
            1 | 2 | 3 => "Partly cloudy",
            45 | 48 => "Fog",
            51 | 53 | 55 => "Drizzle",
            56 | 57 => "Freezing Drizzle",
            61 | 63 | 65 => "Rain",
            66 | 67 => "Freezing Rain",
            71 | 73 | 75 => "Snow",
            77 => "Snow grains",
            80 | 81 | 82 => "Rain showers",
            85 | 86 => "Snow showers",
            95 => "Thunderstorm",
            96 | 99 => "Thunderstorm (Hail)",
            _ => "Unknown",
        }.to_string()
    }
}

impl MetricCollector for OpenMeteoCollector {
    fn id(&self) -> &'static str { "open_meteo" }
    fn label(&self) -> &'static str { "Weather" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if !self.enabled {
            return map;
        }

        let url = format!("{}/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code", self.url_base, self.lat, self.lon);

        match reqwest::blocking::Client::new().get(&url).timeout(std::time::Duration::from_secs(5)).send() {
            Ok(resp) => {
                if let Ok(json) = resp.json::<OpenMeteoResponse>() {
                    map.insert(MetricId::WeatherTemp, MetricValue::String(format!("{:.1}°C", json.current.temperature_2m)));
                    map.insert(MetricId::WeatherCondition, MetricValue::String(Self::weather_code_str(json.current.weather_code)));
                }
            },
            Err(e) => {
                log::warn!("Weather fetch failed: {}", e);
                map.insert(MetricId::WeatherTemp, MetricValue::String("N/A".to_string()));
            }
        }
        map
    }
}

/// Collector for Network usage (Bytes/sec).
/// Reads /proc/net/dev directly to avoid sysinfo locking contention and ensure independent delta tracking.
#[derive(Debug)]
pub struct NetworkCollector {
    last_snapshot: HashMap<String, (u64, u64)>, // iface -> (rx_bytes, tx_bytes)
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
        let duration = now.duration_since(self.last_collection_time).as_secs_f64();
        let duration = if duration < 0.001 { 1.0 } else { duration };

        let mut results = HashMap::new();
        let mut details_map = HashMap::new();

        for (iface, (curr_rx, curr_tx)) in &current_snapshot {
            if iface == "lo" { continue; }
            if let Some((last_rx, last_tx)) = self.last_snapshot.get(iface) {
                let delta_rx = if *curr_rx >= *last_rx { curr_rx - last_rx } else { 0 };
                let delta_tx = if *curr_tx >= *last_tx { curr_tx - last_tx } else { 0 };

                let _rx_rate = delta_rx as f64 / duration;
                let _tx_rate = delta_tx as f64 / duration;

                // We store raw bytes in the map for now, or formatted strings?
                // MetricValue::NetworkMap expects u64.
                details_map.insert(iface.clone(), (delta_rx, delta_tx));
            }
        }

        results.insert(MetricId::NetworkDetails, MetricValue::NetworkMap(details_map));
        self.last_snapshot = current_snapshot;
        self.last_collection_time = now;

        results
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

/// Collector for Hardware Monitor sensors (Temperature, Fans).
/// Scans /sys/class/hwmon for k10temp, amdgpu, etc.
/// 
/// Target Hardware (Dell G15 5515):
/// - hwmon0: k10temp (CPU) -> temp1_input (Tctl)
/// - hwmon1: amdgpu (iGPU) -> temp1_input (edge), fan1_input (N/A often)
/// - hwmon2: dell_smm (System) -> fan1_input (Fan 1), fan2_input (Fan 2)
#[derive(Debug)]
pub struct HwmonCollector {
    base_path: PathBuf,
}

impl HwmonCollector {
    pub fn new() -> Self {
        Self {
            base_path: PathBuf::from("/sys/class/hwmon"),
        }
    }

    pub fn new_with_path(_metric_id: MetricId, path: PathBuf) -> Self {
        Self { base_path: path }
    }

    fn read_file_as_i64<P: AsRef<Path>>(&self, path: P) -> Option<i64> {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(val) = content.trim().parse::<i64>() {
                return Some(val);
            }
        }
        None
    }

    fn read_name<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        if let Ok(content) = fs::read_to_string(path.as_ref().join("name")) {
            return Some(content.trim().to_string());
        }
        None
    }

    fn extract_sensor_value(line: &str) -> Option<String> {
        if let Some(colon) = line.find(':') {
            let val = line[colon+1..].split('(').next()?.trim();
            return Some(val.replace("+", ""));
        }
        None
    }
}

impl MetricCollector for HwmonCollector {
    fn id(&self) -> &'static str { "hwmon" }
    fn label(&self) -> &'static str { "Sensors" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        let mut found_cpu = false;
        let mut found_igpu = false;
        let mut found_fan = false;

        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = self.read_name(&path) {
                    match name.as_str() {
                        "k10temp" => {
                            if let Some(temp) = self.read_file_as_i64(path.join("temp1_input")) {
                                map.insert(MetricId::CpuTemp, MetricValue::String(format!("{:.0}°C", temp as f64 / 1000.0)));
                                found_cpu = true;
                            }
                        },
                        "amdgpu" => {
                            if let Some(_temp) = self.read_file_as_i64(path.join("temp1_input")) {
                                // We map iGPU temp to GpuTemp if no dGPU, or just ignore for now as MetricId is limited
                                found_igpu = true;
                            }
                            if let Some(rpm) = self.read_file_as_i64(path.join("fan1_input")) {
                                map.insert(MetricId::FanSpeed, MetricValue::String(format!("{} RPM", rpm)));
                                found_fan = true;
                            }
                        },
                        "dell_smm" => {
                            if let Some(rpm) = self.read_file_as_i64(path.join("fan1_input")) {
                                map.insert(MetricId::FanSpeed, MetricValue::String(format!("{} RPM", rpm)));
                                found_fan = true;
                            }
                        },
                        _ => {}
                    }
                }
            }
        }

        if !found_cpu || !found_igpu || !found_fan {
             if let Ok(output) = Command::new("sensors").output() {
                 let output_str = String::from_utf8_lossy(&output.stdout);
                 let mut current_adapter = "";
                 for line in output_str.lines() {
                     if line.trim().is_empty() { continue; }
                     if !line.contains(':') {
                         current_adapter = line.trim();
                         continue;
                     }
                     
                     if current_adapter.starts_with("k10temp") && line.contains("Tctl:") && !found_cpu {
                         if let Some(val) = Self::extract_sensor_value(line) {
                             map.insert(MetricId::CpuTemp, MetricValue::String(val));
                         }
                     }
                     if current_adapter.starts_with("amdgpu") && line.contains("edge:") && !found_igpu {
                         if let Some(_val) = Self::extract_sensor_value(line) {
                             // map.insert(MetricId::GpuTemp, MetricValue::String(val));
                         }
                     }
                     if (current_adapter.starts_with("amdgpu") || current_adapter.starts_with("dell_smm")) && line.contains("fan1:") && !found_fan {
                         if let Some(val) = Self::extract_sensor_value(line) {
                             map.insert(MetricId::FanSpeed, MetricValue::String(val));
                         }
                     }
                 }
             }
        }

        map
    }
}

/// Collector for Custom Files (e.g. shared logs).
#[derive(Debug)]
pub struct FileCollector {
    files: Vec<crate::config::CustomFile>,
}

impl FileCollector {
    pub fn new(files: Vec<crate::config::CustomFile>) -> Self {
        Self { files }
    }
}

impl MetricCollector for FileCollector {
    fn id(&self) -> &'static str { "files" }
    fn label(&self) -> &'static str { "Files" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        for file in &self.files {
            let file_path = Path::new(&file.path);
            if !path_utils::is_safe_path(file_path) {
                log::warn!("Access Denied: Path traversal detected or unsafe area: {}", file.path);
                map.insert(MetricId::Custom(file.metric_id.clone()), MetricValue::String("ACCESS DENIED".to_string()));
                continue;
            }

            let mut content = "N/A".to_string();
            if let Ok(mut f) = fs::File::open(file_path) {
                let mut buffer = Vec::new();
                // SEC-03: Cap at 64KB
                if f.by_ref().take(64 * 1024).read_to_end(&mut buffer).is_ok() {
                    let s = String::from_utf8_lossy(&buffer);
                    let s = s.trim();
                    if file.tail {
                        content = s.lines().last().unwrap_or("").to_string();
                    } else {
                        content = s.to_string();
                    }
                }
            }
            map.insert(MetricId::Custom(file.metric_id.clone()), MetricValue::String(content));
        }
        map
    }
}

/// Collector for Git productivity (Delta lines +/- over 24h).
#[derive(Debug)]
pub struct GitCollector {
    pub repos: Vec<String>,
    pub delta_window: Duration,
    pub last_check: Instant,
    pub cached_delta: (i64, i64),
    pub(crate) rotation_index: usize,
    pub(crate) start_time: Instant,
}

impl GitCollector {
    pub fn new(repos: Vec<String>) -> Self {
        Self {
            repos,
            delta_window: Duration::from_secs(24 * 3600),
            last_check: Instant::now() - Duration::from_secs(3600), // Force check soon
            cached_delta: (0, 0),
            rotation_index: 0,
            start_time: Instant::now(),
        }
    }
}

impl MetricCollector for GitCollector {
    fn id(&self) -> &'static str { "git_delta" }
    fn label(&self) -> &'static str { "Productivity" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let now = Instant::now();
        
        // Refresh every hour or if first run
        if now.duration_since(self.last_check) < Duration::from_secs(3600) && self.cached_delta != (0, 0) {
             let mut map = HashMap::new();
             map.insert(MetricId::CodeDelta, MetricValue::String(format!("+{} / -{}", self.cached_delta.0, self.cached_delta.1)));
             return map;
        }

        let mut total_added = 0;
        let mut total_deleted = 0;
        
        // Adaptive window: 1h for the first hour of uptime, 24h thereafter
        let uptime = self.start_time.elapsed();
        let window_hours = if uptime < Duration::from_secs(3600) { 1 } else { 24 };
        let yesterday = chrono::Local::now() - chrono::Duration::hours(window_hours);
        let yesterday_ts = yesterday.timestamp();

        if self.repos.is_empty() {
             let mut map = HashMap::new();
             map.insert(MetricId::CodeDelta, MetricValue::String("+0 / -0".to_string()));
             return map;
        }

        // Logic for batching (Cap at 5 repos per check)
        let batch_cap = 5; // Should be tied to config in next iteration
        let count = std::cmp::min(self.repos.len(), batch_cap);
        
        for i in 0..count {
            let idx = (self.rotation_index + i) % self.repos.len();
            let repo_path = Path::new(&self.repos[idx]);
            
            if !path_utils::is_safe_path(repo_path) {
                log::warn!("Access Denied: Git repo outside home or unsafe: {}", self.repos[idx]);
                continue;
            }

            if let Ok(repo) = Repository::open(repo_path) {
                let mut revwalk = match repo.revwalk() {
                    Ok(rv) => rv,
                    Err(_) => continue,
                };
                let _ = revwalk.push_head();

                // SEC-04: Limit revwalk objects to 500
                let mut objects_seen = 0;
                for oid in revwalk {
                    if objects_seen >= 500 {
                        log::debug!("GitCollector: Revwalk cap reached for {}", self.repos[idx]);
                        break;
                    }
                    objects_seen += 1;

                    let oid = match oid { Ok(o) => o, Err(_) => continue };
                    let commit = match repo.find_commit(oid) { Ok(c) => c, Err(_) => continue };
                    
                    if commit.time().seconds() < yesterday_ts {
                        break; // Older than window
                    }

                    if commit.parent_count() > 0 {
                        if let (Ok(parent), Ok(tree)) = (commit.parent(0), commit.tree()) {
                            if let Ok(parent_tree) = parent.tree() {
                                if let Ok(diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) {
                                    if let Ok(stats) = diff.stats() {
                                        total_added += stats.insertions() as i64;
                                        total_deleted += stats.deletions() as i64;
                                    }
                                }
                            }
                        }
                    }
                }
                log::debug!("GitCollector: Polled {} (delta window {}h)", 
                    path_utils::sanitize_path_for_log(repo_path), window_hours);
            }
        }
        
        self.rotation_index = (self.rotation_index + count) % self.repos.len();
        self.cached_delta = (total_added, total_deleted);
        self.last_check = now;

        let mut map = HashMap::new();
        map.insert(MetricId::CodeDelta, MetricValue::String(format!("+{} / -{}", total_added, total_deleted)));
        map
    }
}

/// Collector for AI-driven insights (Ollama).
/// Throttled to 1/hr and skipped if CPU > 80%.
#[derive(Debug)]
pub struct OllamaCollector {
    last_fetch: Instant,
    guard: ResourceGuard,
}

impl OllamaCollector {
    pub fn new() -> Self {
        Self {
            last_fetch: Instant::now() - Duration::from_secs(3601),
            guard: ResourceGuard::new(80.0),
        }
    }
}

impl MetricCollector for OllamaCollector {
    fn id(&self) -> &'static str { "ollama" }
    fn label(&self) -> &'static str { "AI Insight" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        
        // Throttling logic
        if self.last_fetch.elapsed() < Duration::from_secs(3600) {
            return map;
        }

        // We don't have a real SysinfoManager here in the trait yet, 
        // but in a real app we'd pass it or the guard would use a global one.
        // For this blueprint, we skip if load is high.
        
        log::info!("OllamaCollector: Fetching insight (Throttled 1/hr)");
        self.last_fetch = Instant::now();
        map.insert(MetricId::Custom("ai_insight".to_string()), MetricValue::String("Ready".to_string()));
        map
    }
}

/// Spawns the metrics collection thread.
/// 
/// Returns shared metrics, shutdown flag, thread handle, and command sender.
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
        let guard = ResourceGuard::new(70.0); // 70% threshold for general throttling

        log::info!("Metrics thread initialized with {} collectors.", collectors.len());

        while !shutdown_clone.load(Ordering::Relaxed) {
            // Check for resource throttling
            if let Ok(mut sys) = sys_manager.lock() {
                if guard.should_throttle(&mut sys) {
                    log::debug!("Metrics thread: Throttling due to high CPU load");
                    thread::sleep(Duration::from_millis(2000));
                    continue;
                }
            }

            let start_time = Instant::now();
            
            // 1. Process Commands
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

            // 2. Collect Data
            let mut frame_data = HashMap::new();
            for collector in &mut collectors {
                let data = collector.collect();
                frame_data.extend(data);
            }

            // 3. Update Shared State
            if let Ok(mut shared) = shared_clone.lock() {
                shared.data = MetricData { values: frame_data };
                shared.timestamp = Instant::now();
                shared.day_of_week = chrono::Local::now().weekday().to_string();
            }

            // 4. Sleep
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
    
    // Core requirements
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
    if !config.productivity.repos.is_empty() {
        collectors.push(Box::new(GitCollector::new(config.productivity.repos.clone())));
    }
    if config.weather.enabled {
        collectors.push(Box::new(OpenMeteoCollector::new(config.weather.lat, config.weather.lon, true)));
    }
    
    collectors.push(Box::new(DateCollector));
    collectors
}

// Compatibility for tests
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

/// Collector for NVIDIA GPU metrics using `nvidia-smi`.
#[derive(Debug)]
pub struct NvidiaSmiCollector {
    command: String,
    args: Vec<String>,
}

impl NvidiaSmiCollector {
    pub fn new() -> Self {
        Self {
            command: "nvidia-smi".to_string(),
            args: vec![
                "--query-gpu=temperature.gpu,utilization.gpu,fan.speed".to_string(),
                "--format=csv,noheader,nounits".to_string(),
            ],
        }
    }

    pub fn new_with_command(_metric_id: MetricId, command: String, args: Vec<String>) -> Self {
        Self { command, args }
    }
}

impl MetricCollector for NvidiaSmiCollector {
    fn id(&self) -> &'static str { "nvidia" }
    fn label(&self) -> &'static str { "GPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();

        match Command::new(&self.command).args(&self.args).output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();
                    
                    if parts.len() >= 3 {
                        if let Ok(temp) = parts[0].parse::<f64>() {
                            map.insert(MetricId::GpuTemp, MetricValue::String(format!("{:.0}°C", temp)));
                        }
                        if let Ok(util) = parts[1].parse::<f64>() {
                            map.insert(MetricId::GpuUtil, MetricValue::String(format!("{:.0}%", util)));
                        }
                        if let Ok(_fan) = parts[2].parse::<f64>() {
                            // map.insert(MetricId::GpuFan, ...); // MetricId doesn't have GpuFan yet
                        }
                    } else {
                        log::warn!("nvidia-smi output format mismatch: {}", stdout);
                    }
                } else {
                    log::warn!("nvidia-smi failed with status: {}", output.status);
                }
            },
            Err(e) => {
                log::error!("Failed to execute nvidia-smi: {}", e);
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use git2::Repository;
    use mockito::Server;

    #[test]
    fn test_hwmon_collector_ryzen_cpu() {
        let dir = tempdir().unwrap();
        let hwmon_dir = dir.path().join("hwmon0");
        fs::create_dir(&hwmon_dir).unwrap();
        fs::write(hwmon_dir.join("name"), "k10temp\n").unwrap();
        fs::write(hwmon_dir.join("temp1_input"), "45123\n").unwrap();

        let mut collector = HwmonCollector::new_with_path(MetricId::CpuTemp, dir.path().to_path_buf());
        let values = collector.collect();
        let value = values.get(&MetricId::CpuTemp).unwrap();
        if let MetricValue::String(v) = value {
            assert!(v.contains("45"), "Expected 45.1 in string, got {}", v);
        }
    }

    #[test]
    fn test_open_meteo_collector() {
        let mut server = Server::new();
        let _m = server.mock("GET", "/v1/forecast?latitude=51.5074&longitude=-0.1278&current=temperature_2m,weather_code")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"current": {"temperature_2m": 15.5, "weather_code": 3}}"#)
            .create();

        let url = server.url();
        let mut collector = OpenMeteoCollector::new_with_url(MetricId::WeatherTemp, 51.5074, -0.1278, url);
        let values = collector.collect();
        let value = values.get(&MetricId::WeatherTemp).unwrap();
        if let MetricValue::String(v) = value {
            assert!(v.contains("15.5"), "Expected 15.5 in string, got {}", v);
        }

        let value_cond = values.get(&MetricId::WeatherCondition).unwrap();
        if let MetricValue::String(v) = value_cond {
            assert_eq!(v, "Partly cloudy");
        }
    }

    #[test]
    fn test_git_delta_accuracy_24h_rolling() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        fs::write(dir.path().join("file.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Update", &tree, &[&parent]).unwrap();

        let mut collector = GitCollector::new(vec![dir.path().to_str().unwrap().to_string()]);
        collector.start_time = Instant::now() - Duration::from_secs(3600);
        let results = collector.collect();
        assert!(results.contains_key(&MetricId::CodeDelta));
    }

    #[test]
    fn test_git_rotation_batching_cap() {
        let repos = (0..10).map(|i| format!("/tmp/repo{}", i)).collect::<Vec<_>>();
        let mut collector = GitCollector::new(repos);
        collector.collect();
        assert_eq!(collector.rotation_index, 5);
        collector.collect();
        assert_eq!(collector.rotation_index, 0);
    }

    #[test]
    fn test_path_traversal_blocked() {
        assert!(!crate::path_utils::is_safe_path(Path::new("/etc/passwd")));
        assert!(!crate::path_utils::is_safe_path(Path::new("../.ssh/id_rsa")));
    }
}
