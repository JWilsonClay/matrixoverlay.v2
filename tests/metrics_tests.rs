use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use matrix_overlay::metrics::{
    MetricCollector, MetricId, MetricValue, HwmonCollector, NvidiaSmiCollector, 
    OpenMeteoCollector, SysinfoCollector, SysinfoManager
};

#[test]
fn test_hwmon_collector_ryzen_cpu() {
    // Setup mock filesystem for Ryzen 5800H (k10temp)
    let dir = tempdir().unwrap();
    let hwmon_dir = dir.path().join("hwmon0");
    fs::create_dir(&hwmon_dir).unwrap();

    let name_path = hwmon_dir.join("name");
    let mut name_file = File::create(name_path).unwrap();
    write!(name_file, "k10temp\n").unwrap();

    let temp_path = hwmon_dir.join("temp1_input");
    let mut temp_file = File::create(temp_path).unwrap();
    write!(temp_file, "45123\n").unwrap(); // 45.123 C

    let mut collector = HwmonCollector::new_with_path(MetricId::CpuTemp, dir.path().to_path_buf());
    let map = collector.collect();

    if let Some(MetricValue::String(v)) = map.get(&MetricId::CpuTemp) {
        assert_eq!(v, "45°C");
    } else {
        panic!("Expected String value for CpuTemp, got {:?}", map);
    }
}

#[test]
fn test_hwmon_collector_fan() {
    // Setup mock filesystem for Fan (dell_smm or similar)
    let dir = tempdir().unwrap();
    let hwmon_dir = dir.path().join("hwmon1");
    fs::create_dir(&hwmon_dir).unwrap();

    let name_path = hwmon_dir.join("name");
    let mut name_file = File::create(name_path).unwrap();
    write!(name_file, "dell_smm\n").unwrap();

    let fan_path = hwmon_dir.join("fan1_input");
    let mut fan_file = File::create(fan_path).unwrap();
    write!(fan_file, "2400\n").unwrap();

    let mut collector = HwmonCollector::new_with_path(MetricId::FanSpeed, dir.path().to_path_buf());
    let map = collector.collect();

    if let Some(MetricValue::String(v)) = map.get(&MetricId::FanSpeed) {
        assert_eq!(v, "2400 RPM");
    } else {
        panic!("Expected String value, got {:?}", map);
    }
}

#[test]
fn test_nvidia_collector_parsing() {
    // We use `cat` to output our mock file content, simulating nvidia-smi output
    let mock_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/test_data/nvidia_mock.txt");
    
    // Ensure the mock file exists
    assert!(mock_file_path.exists(), "Mock file missing at {:?}", mock_file_path);

    let mut collector = NvidiaSmiCollector::new_with_command(
        MetricId::GpuTemp,
        "cat".to_string(),
        vec![mock_file_path.to_string_lossy().to_string()]
    );

    let map = collector.collect();
    if let Some(MetricValue::String(v)) = map.get(&MetricId::GpuTemp) {
        assert_eq!(v, "45°C");
    } else {
        panic!("Expected String(45°C) for GPU Temp, got {:?}", map);
    }

    // Test Utilization from same file
    let mut collector_util = NvidiaSmiCollector::new_with_command(
        MetricId::GpuUtil,
        "cat".to_string(),
        vec![mock_file_path.to_string_lossy().to_string()]
    );

    let map_util = collector_util.collect();
    if let Some(MetricValue::String(v)) = map_util.get(&MetricId::GpuUtil) {
        assert_eq!(v, "20%");
    } else {
        panic!("Expected String(20%) for GPU Util, got {:?}", map_util);
    }
}

#[test]
fn test_open_meteo_collector() {
    let mut server = mockito::Server::new();
    let _m = server.mock("GET", "/v1/forecast?latitude=51.5074&longitude=-0.1278&current=temperature_2m,weather_code")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"current": {"temperature_2m": 15.5, "weather_code": 3}}"#)
        .create();

    let url = server.url();
    let mut collector = OpenMeteoCollector::new_with_url(
        MetricId::WeatherTemp, 
        51.5074, 
        -0.1278, 
        url.clone()
    );

    let map = collector.collect();
    
    match map.get(&MetricId::WeatherTemp) {
        Some(MetricValue::String(s)) => assert_eq!(s, "15.5°C"),
        _ => panic!("Expected weather_temp string, got {:?}", map.get(&MetricId::WeatherTemp)),
    }
    
    // Code 3 -> "Partly cloudy"
    match map.get(&MetricId::WeatherCondition) {
        Some(MetricValue::String(s)) => assert_eq!(s, "Partly cloudy"),
        _ => panic!("Expected weather_cond string, got {:?}", map.get(&MetricId::WeatherCondition)),
    }
}

#[test]
fn test_sysinfo_collector_defaults() {
    // We can't easily mock sysinfo::System without a trait, but we can verify
    // the collector runs against the real system without panicking and returns valid types.
    let manager = Arc::new(Mutex::new(SysinfoManager::new()));
    
    let mut cpu_collector = SysinfoCollector::new(MetricId::CpuUsage, manager.clone());
    let cpu_map = cpu_collector.collect();
    if let Some(MetricValue::Float(v)) = cpu_map.get(&MetricId::CpuUsage) {
        assert!(*v >= 0.0 && *v <= 100.0, "CPU usage {} out of range", *v);
    } else {
        panic!("CPU Usage should be float");
    }

    let mut ram_collector = SysinfoCollector::new(MetricId::RamUsage, manager.clone());
    let ram_map = ram_collector.collect();
    if let Some(MetricValue::Float(v)) = ram_map.get(&MetricId::RamUsage) {
        assert!(*v >= 0.0 && *v <= 100.0, "RAM usage {} out of range", *v);
    } else {
        panic!("RAM Usage should be float");
    }

    let mut uptime_collector = SysinfoCollector::new(MetricId::Uptime, manager.clone());
    let uptime_map = uptime_collector.collect();
    if let Some(MetricValue::Int(v)) = uptime_map.get(&MetricId::Uptime) {
        assert!(*v > 0, "Uptime should be positive");
    } else {
        panic!("Uptime should be int");
    }
}
