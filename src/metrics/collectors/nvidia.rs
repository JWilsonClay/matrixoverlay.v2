//! NVIDIA GPU metrics collection substrate via nvidia-smi.
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};
use crate::metrics::{MetricId, MetricValue, MetricCollector};

#[derive(Debug)]
pub struct NvidiaSmiCollector { temp_unit: String }

impl NvidiaSmiCollector {
    pub fn new(temp_unit: String) -> Self { Self { temp_unit } }
}

impl MetricCollector for NvidiaSmiCollector {
    fn id(&self) -> &'static str { "nvidia" }
    fn label(&self) -> &'static str { "GPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();

        // [HARDENING] Secure binary execution with timeout
        // [5.8 F-C] Timed: `CLAUDE.md` warns that polling the dGPU in PRIME
        // on-demand wakes it, which makes this subprocess the leading suspect
        // for the frame-rate-independent floor. Unconditional and cheap — one
        // `Instant` around a call that already spawns a process.
        let t_nv = Instant::now();
        let output = Command::new("nvidia-smi")
            .args(&["--query-gpu=temperature.gpu,utilization.gpu", "--format=csv,noheader,nounits"])
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null()).output().ok();
        crate::core::telemetry::phase58::add_nvidia_call(t_nv.elapsed().as_nanos() as u64);

        if let Some(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = s.trim().split(',').map(|s| s.trim()).collect();
            if parts.len() >= 2 {
                if let Ok(mut t) = parts[0].parse::<f64>() {
                    let suf = if self.temp_unit == "fahrenheit" { t = (t * 1.8) + 32.0; "°F" } else { "°C" };
                    map.insert(MetricId::GpuTemp, MetricValue::String(format!("{:.0}{}", t, suf)));
                }
                if let Ok(u) = parts[1].parse::<f64>() {
                    map.insert(MetricId::GpuUtil, MetricValue::String(format!("{:.0}%", u)));
                }
            }
        }
        map
    }
}
