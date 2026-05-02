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

        match Command::new(&self.command).args(&self.args).output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();
                    
                    if parts.len() >= 3 {
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
