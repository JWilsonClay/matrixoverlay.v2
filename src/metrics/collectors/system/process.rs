// src/metrics/collectors/system/process.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::{SystemExt, ProcessExt, get_current_pid, Pid};
use crate::metrics::{MetricId, MetricValue, MetricCollector, SysinfoManager};

#[derive(Debug)]
pub struct OverlayCpuCollector {
    sys: Arc<Mutex<SysinfoManager>>,
    pid: Pid,
}

impl OverlayCpuCollector {
    pub fn new(sys: Arc<Mutex<SysinfoManager>>) -> Self {
        Self { sys, pid: get_current_pid().unwrap_or(Pid::from(0)) }
    }
}

/// Formats `sysinfo`'s raw process CPU reading for display.
///
/// This is the single place the `overlay_cpu` normalization decision lives, and
/// `collect()` below is a one-line delegation to it — so a future change to the
/// rule has to happen *here*, where the tests are, rather than silently upstream.
///
/// The rule is identity: `sysinfo` already returns percent-of-one-core.
pub(crate) fn format_overlay_cpu(raw_cpu_percent: f32) -> String {
    format!("{:.2}%", raw_cpu_percent)
}

impl MetricCollector for OverlayCpuCollector {
    fn id(&self) -> &'static str { "overlay_cpu" }
    fn label(&self) -> &'static str { "Overlay CPU" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if let Ok(mut manager) = self.sys.lock() {
            manager.system.refresh_process(self.pid);
            if let Some(p) = manager.system.process(self.pid) {
                // [F2] `sysinfo` returns process CPU as a percentage of ONE core
                // already — a process saturating two cores reads ~200%. Its own
                // docs (traits.rs:358) advise dividing by the CPU count, and this
                // collector used to. We deliberately do NOT.
                //
                // Dividing yields percent-of-machine. On this 16-core host that
                // under-reports by 16x: the 2026-09-03 investigation found the
                // overlay at 60.7% of one core while this metric displayed 3.79%,
                // which is why a 20-60x budget overrun ran unnoticed for ~24h.
                //
                // The on-screen label is "HUD CPU" (see MetricId::label in
                // metrics/mod.rs), which invites comparison against an external
                // per-core reading — Method M-1 in the implementation plan, or
                // `top`. Both report percent-of-one-core. The metric must match
                // the instrument a reader will check it against.
                //
                // Do not reinstate the division to "follow the sysinfo docs".
                // If percent-of-machine is ever wanted, surface it as a distinct,
                // differently-labeled metric rather than redefining this one.
                map.insert(MetricId::OverlayCpu, MetricValue::String(format_overlay_cpu(p.cpu_usage())));
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::format_overlay_cpu;

    /// [1.6 / S-03] The collector must report percent of ONE core — what Method
    /// M-1 and `top` report — not percent of the whole machine.
    ///
    /// These call the production `format_overlay_cpu` directly. An earlier draft
    /// of this test defined a local copy of the formatting expression and
    /// asserted against that; it would have passed unchanged if `/ cores` were
    /// reinstated. That is precisely the Mock Trap this campaign exists to
    /// remove, and it must not be reintroduced here.
    ///
    /// Residual gap, stated rather than hidden: these assertions bind the
    /// normalization *function*. They cannot catch a division reinstated in
    /// `collect()` before the call. That is why `collect()` is kept a one-line
    /// delegation — the workaround has to be visible to be possible.
    #[test]
    fn reports_percent_of_one_core_not_of_machine() {
        // The live defect: 60.7% of one core on a 16-core host.
        assert_eq!(format_overlay_cpu(60.7), "60.70%");

        // What the old `/ cores` behaviour displayed instead: 60.7 / 16 = 3.79%,
        // the exact figure that hid a 20-60x budget overrun for ~24 hours.
        assert_ne!(format_overlay_cpu(60.7), "3.79%");
    }

    #[test]
    fn can_exceed_one_hundred_percent() {
        // A process saturating two cores reads ~200% of one core. Clamping to
        // 100 would hide overrun the same way dividing did.
        assert_eq!(format_overlay_cpu(203.4), "203.40%");
    }

    #[test]
    fn handles_idle() {
        assert_eq!(format_overlay_cpu(0.0), "0.00%");
    }
}
