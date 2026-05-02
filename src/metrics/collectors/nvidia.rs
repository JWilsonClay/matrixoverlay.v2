use std::collections::HashMap;
use std::process::Command;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

/// Collector for NVIDIA GPU metrics using `nvidia-smi`.
#[derive(Debug)]
pub struct NvidiaSmiCollector {
    command: String,
    args: Vec<String>,
    temp_unit: String,
}

impl NvidiaSmiCollector {
    pub fn new(temp_unit: String) -> Self {
        Self {
            command: "nvidia-smi".to_string(),
            args: vec![
                "--query-gpu=temperature.gpu,utilization.gpu,fan.speed".to_string(),
                "--format=csv,noheader,nounits".to_string(),
            ],
            temp_unit,
        }
    }

    pub fn new_with_command(_metric_id: MetricId, command: String, args: Vec<String>) -> Self {
        Self { command, args, temp_unit: "celsius".to_string() }
    }
}

impl MetricCollector for NvidiaSmiCollector {
    fn id(&self) -> &'static str { "nvidia" }
    fn label(&self) -> &'static str { "GPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();

        // **[HARDENING: Command Timeout]**
        // Using a 2-second timeout for nvidia-smi to prevent blocking.
        use std::time::Duration;
        use std::thread;

        let child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn();

        if let Ok(mut child) = child {
            let timeout = Duration::from_secs(2);
            let start = std::time::Instant::now();
            let mut output = Vec::new();

            loop {
                if let Ok(Some(_status)) = child.try_wait() {
                    if let Some(mut stdout) = child.stdout.take() {
                        use std::io::Read;
                        let _ = stdout.read_to_end(&mut output);
                    }
                    break;
                }
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    log::warn!("nvidia-smi command timed out");
                    return map;
                }
                thread::sleep(Duration::from_millis(10));
            }

            let stdout = String::from_utf8_lossy(&output);
            let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();
            
            if parts.len() >= 2 {
                if let Ok(mut temp) = parts[0].parse::<f64>() {
                    let mut suffix = "°C";
                    if self.temp_unit == "fahrenheit" {
                        temp = (temp * 9.0 / 5.0) + 32.0;
                        suffix = "°F";
                    }
                    map.insert(MetricId::GpuTemp, MetricValue::String(format!("{:.0}{}", temp, suffix)));
                }
                if let Ok(util) = parts[1].parse::<f64>() {
                    map.insert(MetricId::GpuUtil, MetricValue::String(format!("{:.0}%", util)));
                }
            } else {
                log::warn!("nvidia-smi output format mismatch: {}", stdout);
            }
        }
        map
    }
}
