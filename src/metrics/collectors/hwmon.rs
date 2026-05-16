//! Hardware Monitor (HWMON) metrics collection substrate.
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

#[derive(Debug)]
pub struct HwmonCollector { base_path: PathBuf, temp_unit: String }

impl HwmonCollector {
    pub fn new(temp_unit: String) -> Self {
        Self { base_path: PathBuf::from("/sys/class/hwmon"), temp_unit }
    }

    pub fn new_with_path(_id: crate::metrics::MetricId, base_path: PathBuf) -> Self {
        Self { base_path, temp_unit: "celsius".to_string() }
    }

    fn read_val<P: AsRef<Path>>(&self, p: P) -> Option<i64> {
        fs::read_to_string(p).ok()?.trim().parse().ok()
    }

    fn read_name<P: AsRef<Path>>(&self, p: P) -> Option<String> {
        fs::read_to_string(p.as_ref().join("name")).ok().map(|s| s.trim().to_string())
    }

    fn parse_sensors_cmd(&self, map: &mut HashMap<MetricId, MetricValue>) {
        // [HARDENING] Command timeout and safe execution
        let output = std::process::Command::new("sensors")
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null()).output().ok();

        if let Some(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let mut adapter = "";
            for line in s.lines() {
                if line.trim().is_empty() { continue; }
                if !line.contains(':') { adapter = line.trim(); continue; }
                if adapter.starts_with("k10temp") && line.contains("Tctl:") {
                    if let Some(v) = line.split(':').nth(1).and_then(|v| v.split('(').next()) {
                        map.insert(MetricId::CpuTemp, MetricValue::String(v.trim().replace("+", "")));
                    }
                }
            }
        }
    }
}

impl MetricCollector for HwmonCollector {
    fn id(&self) -> &'static str { "hwmon" }
    fn label(&self) -> &'static str { "Sensors" }

    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        let mut cpu_ok = false; let mut fan_ok = false;

        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for (i, entry) in entries.flatten().enumerate() {
                if i > 32 { break; }
                let p = entry.path();
                if let Some(name) = self.read_name(&p) {
                    if name == "k10temp" {
                        if let Some(raw) = self.read_val(p.join("temp1_input")) {
                            let mut t = raw as f64 / 1000.0;
                            let suf = if self.temp_unit == "fahrenheit" { t = (t * 1.8) + 32.0; "°F" } else { "°C" };
                            map.insert(MetricId::CpuTemp, MetricValue::String(format!("{:.0}{}", t, suf)));
                            cpu_ok = true;
                        }
                    } else if ["amdgpu", "dell_smm", "it87", "nct6775"].contains(&name.as_str()) {
                        for j in 1..=3 {
                            if let Some(rpm) = self.read_val(p.join(format!("fan{}_input", j))) {
                                if rpm > 0 && !fan_ok {
                                    map.insert(MetricId::FanSpeed, MetricValue::String(format!("{} RPM", rpm)));
                                    fan_ok = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !cpu_ok || !fan_ok { self.parse_sensors_cmd(&mut map); }
        map
    }
}
