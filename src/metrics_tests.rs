use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use mockito::mock;
use x11_monitor_overlay::metrics::{
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
    let value = collector.collect();

    if let MetricValue::Float(v) = value {
        assert!((v - 45.123).abs() < 0.001, "Expected 45.123, got {}", v);
    } else {
        panic!("Expected Float value, got {:?}", value);
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
    let value = collector.collect();

    if let MetricValue::Int(v) = value {
        assert_eq!(v, 2400);
    } else {
        panic!("Expected Int value, got {:?}", value);
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

    let value = collector.collect();
    if let MetricValue::Float(v) = value {
        assert_eq!(v, 45.0);
    } else {
        panic!("Expected Float(45.0) for GPU Temp, got {:?}", value);
    }

    // Test Utilization from same file
    let mut collector_util = NvidiaSmiCollector::new_with_command(
        MetricId::GpuUtil,
        "cat".to_string(),
        vec![mock_file_path.to_string_lossy().to_string()]
    );

    let value_util = collector_util.collect();
    if let MetricValue::Float(v) = value_util {
        assert_eq!(v, 20.0);
    } else {
        panic!("Expected Float(20.0) for GPU Util, got {:?}", value_util);
    }
}

#[test]
fn test_open_meteo_collector() {
    let _m = mock("GET", "/v1/forecast?latitude=51.5074&longitude=-0.1278&current_weather=true")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"current_weather": {"temperature": 15.5, "weathercode": 3}}"#)
        .create();

    let url = mockito::server_url();
    let mut collector = OpenMeteoCollector::new_with_url(
        MetricId::WeatherTemp, 
        51.5074, 
        -0.1278, 
        url.clone()
    );

    let value = collector.collect();
    if let MetricValue::Float(v) = value {
        assert_eq!(v, 15.5);
    } else {
        panic!("Expected Float(15.5), got {:?}", value);
    }

    let mut collector_code = OpenMeteoCollector::new_with_url(
        MetricId::WeatherCondition, 
        51.5074, 
        -0.1278, 
        url
    );

    let value_code = collector_code.collect();
    if let MetricValue::Int(v) = value_code {
        assert_eq!(v, 3);
    } else {
        panic!("Expected Int(3), got {:?}", value_code);
    }
}

#[test]
fn test_sysinfo_collector_defaults() {
    // We can't easily mock sysinfo::System without a trait, but we can verify
    // the collector runs against the real system without panicking and returns valid types.
    let manager = Arc::new(Mutex::new(SysinfoManager::new()));
    
    let mut cpu_collector = SysinfoCollector::new(MetricId::CpuUsage, manager.clone());
    let cpu_val = cpu_collector.collect();
    if let MetricValue::Float(v) = cpu_val {
        assert!(v >= 0.0 && v <= 100.0, "CPU usage {} out of range", v);
    } else {
        panic!("CPU Usage should be float");
    }

    let mut ram_collector = SysinfoCollector::new(MetricId::RamUsage, manager.clone());
    let ram_val = ram_collector.collect();
    if let MetricValue::Float(v) = ram_val {
        assert!(v >= 0.0 && v <= 100.0, "RAM usage {} out of range", v);
    } else {
        panic!("RAM Usage should be float");
    }

    let mut uptime_collector = SysinfoCollector::new(MetricId::Uptime, manager.clone());
    let uptime_val = uptime_collector.collect();
    if let MetricValue::Int(v) = uptime_val {
        assert!(v > 0, "Uptime should be positive");
    } else {
        panic!("Uptime should be int");
    }
}