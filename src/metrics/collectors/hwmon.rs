use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

/// Collector for Hardware Monitor sensors (Temperature, Fans).
#[derive(Debug)]
pub struct HwmonCollector {
    base_path: PathBuf,
    temp_unit: String,
}

impl HwmonCollector {
    pub fn new(temp_unit: String) -> Self {
        Self {
            base_path: PathBuf::from("/sys/class/hwmon"),
            temp_unit,
        }
    }

    pub fn new_with_path(_metric_id: MetricId, path: PathBuf) -> Self {
        Self { base_path: path, temp_unit: "celsius".to_string() }
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
                            if let Some(temp_raw) = self.read_file_as_i64(path.join("temp1_input")) {
                                let mut temp = temp_raw as f64 / 1000.0;
                                let mut suffix = "°C";
                                if self.temp_unit == "fahrenheit" {
                                    temp = (temp * 9.0 / 5.0) + 32.0;
                                    suffix = "°F";
                                }
                                map.insert(MetricId::CpuTemp, MetricValue::String(format!("{:.0}{}", temp, suffix)));
                                found_cpu = true;
                            }
                        },
                        "amdgpu" | "dell_smm" | "alienware_wmi" | "it87" | "nct6775" => {
                            for i in 1..=5 {
                                let fan_file = path.join(format!("fan{}_input", i));
                                if let Some(rpm) = self.read_file_as_i64(&fan_file) {
                                    if rpm > 0 {
                                        if !found_fan {
                                            map.insert(MetricId::FanSpeed, MetricValue::String(format!("{} RPM", rpm)));
                                            found_fan = true;
                                        }
                                    }
                                }
                            }
                            
                            if name == "amdgpu" {
                                found_igpu = true;
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
                         // Logic for iGPU if needed
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
}
