//! Hardware-specific integration tests for Dell G15 5515.
//! Covers NVIDIA GPU access, AMD iGPU detection, Fan sensors, and system resilience.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;
use xcb::x;

use matrix_overlay::config::Config;
use matrix_overlay::metrics::{
    HwmonCollector, MetricId, MetricCollector, NvidiaSmiCollector, 
    SysinfoCollector, SysinfoManager, MetricValue
};
use matrix_overlay::window::create_all_windows;

/// Confirm NVIDIA GPU access via nvidia-smi (Target: RTX 3050 Ti)
#[test]
fn test_nvidia_gpu_access() {
    if Command::new("nvidia-smi").arg("-L").output().is_err() {
        eprintln!("Skipping NVIDIA test: nvidia-smi binary not found.");
        return;
    }

    let mut collector = NvidiaSmiCollector::new();
    let map = collector.collect();
    
    match map.get(&MetricId::GpuTemp) {
        Some(MetricValue::String(s)) => {
            // Nvidia collector returns strings like "45°C"
            println!("NVIDIA GPU Temp detected: {}", s);
            assert!(s.contains("°C"), "Expected temp string, got {}", s);
        },
        Some(MetricValue::Float(v)) => {
            assert!(*v > 0.0 && *v < 120.0, "GPU Temp {:.1}°C out of expected range (0-120)", *v);
            println!("NVIDIA GPU Temp detected: {:.1}°C", *v);
        },
        _ => panic!("Failed to collect NVIDIA GPU Temp or invalid type: {:?}", map),
    }
}

/// Confirm AMD iGPU detection in hwmon (Target: Ryzen 5800H)
#[test]
fn test_amd_igpu_detection() {
    let hwmon = Path::new("/sys/class/hwmon");
    if !hwmon.exists() {
        eprintln!("Skipping AMD iGPU test: /sys/class/hwmon not found.");
        return;
    }

    let mut found = false;
    if let Ok(entries) = fs::read_dir(hwmon) {
        for entry in entries.flatten() {
            let name_path = entry.path().join("name");
            if let Ok(name) = fs::read_to_string(name_path) {
                if name.trim() == "amdgpu" {
                    found = true;
                    println!("AMD iGPU found at {:?}", entry.path());
                    break;
                }
            }
        }
    }
    
    assert!(found, "AMD iGPU (amdgpu) not found in hwmon. Is the kernel module loaded?");
}

/// Test Fan Speed reading (Dell SMM or similar)
#[test]
fn test_fan_readings() {
    let mut collector = HwmonCollector::new();
    let map = collector.collect();
    
    if let Some(MetricValue::String(rpm_str)) = map.get(&MetricId::FanSpeed) {
        println!("Fan Speed detected: {}", rpm_str);
        // Simple check if it looks like a reading
        assert!(rpm_str.contains("RPM") || rpm_str.parse::<i64>().is_ok(), "Invalid fan format");
    } else {
        println!("Warning: No fan sensors detected (MetricValue::None). This is common if dell-smm-hwmon is missing.");
    }
}

/// Test resilience under High CPU Load
#[test]
fn test_high_cpu_load_resilience() {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    
    // Spawn a thread to generate CPU load
    let load_thread = thread::spawn(move || {
        while !stop_clone.load(Ordering::Relaxed) {
            let _ = (2.0f64).sqrt() * (3.0f64).sin();
        }
    });

    let start = Instant::now();
    let manager = Arc::new(Mutex::new(SysinfoManager::new()));
    let mut collector = SysinfoCollector::new(MetricId::CpuUsage, manager);
    
    // Perform collection
    let _ = collector.collect();
    let duration = start.elapsed();

    stop.store(true, Ordering::Relaxed);
    load_thread.join().unwrap();

    println!("CPU Metric collection took {:?}", duration);
    assert!(duration < Duration::from_millis(500), "Collection too slow under load (>500ms)");
}

/// Test resilience under High Disk I/O
#[test]
fn test_high_disk_io_resilience() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("io_test.dat");
    
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();

    // Spawn thread to saturate I/O
    let io_thread = thread::spawn(move || {
        if let Ok(mut file) = File::create(file_path) {
            let buffer = [0u8; 1024 * 1024]; // 1MB chunks
            while !stop_clone.load(Ordering::Relaxed) {
                let _ = file.write(&buffer);
                let _ = file.sync_all();
            }
        }
    });

    let start = Instant::now();
    let manager = Arc::new(Mutex::new(SysinfoManager::new()));
    let mut collector = SysinfoCollector::new(MetricId::DiskUsage, manager);
    
    let _ = collector.collect();
    let duration = start.elapsed();

    stop.store(true, Ordering::Relaxed);
    let _ = io_thread.join();

    println!("Disk Metric collection took {:?}", duration);
    assert!(duration < Duration::from_secs(2), "Disk collection too slow under I/O (>2s)");
}

/// Test Window Position Stability (Drift Check)
/// Requires active X11 session.
#[test]
fn test_window_position_stability() {
    let conn_res = xcb::Connection::connect(None);
    if conn_res.is_err() {
        eprintln!("Skipping window stability test: No X11 connection.");
        return;
    }
    let (conn, _screen_num) = conn_res.unwrap();
    let config = Config::default();

    let wm = create_all_windows(&conn, &config).expect("Failed to create windows");
    
    if let Some(monitor) = wm.monitors.first() {
        let win = monitor.window;
        let cookie_start = conn.send_request(&x::GetGeometry { drawable: x::Drawable::Window(win) });
        let geom_start = conn.wait_for_reply(cookie_start).unwrap();
        
        thread::sleep(Duration::from_millis(500));
        
        let cookie_end = conn.send_request(&x::GetGeometry { drawable: x::Drawable::Window(win) });
        let geom_end = conn.wait_for_reply(cookie_end).unwrap();
        
        assert_eq!(geom_start.x(), geom_end.x(), "Window X position drifted!");
        assert_eq!(geom_start.y(), geom_end.y(), "Window Y position drifted!");
    }
}