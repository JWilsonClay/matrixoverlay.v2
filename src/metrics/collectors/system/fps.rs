// src/metrics/collectors/system/fps.rs
//! Frame-rate collector (Phase 1 — S-06).
//!
//! Reports presents per second, derived from `core::telemetry::PRESENT_COUNT`,
//! which the presentation layer increments once per successful
//! `Presenter::present`.
//!
//! This is a *collector* rather than a direct write into `SharedMetrics` for a
//! specific reason (task 1.4b): `metrics::manager` replaces `SharedMetrics.data`
//! wholesale on every collection tick —
//! `sh.data = MetricData { values: frame }` — and `frame` is built only from the
//! collector list. A value published straight into `SharedMetrics` would be
//! erased on the next tick. Publishing through a collector puts it *inside*
//! `frame` instead of being overwritten by it.

use std::collections::HashMap;
use std::time::Instant;
use crate::core::telemetry;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

#[derive(Debug)]
pub struct FpsCollector {
    last_count: u64,
    last_sample: Instant,
}

impl FpsCollector {
    pub fn new() -> Self {
        Self { last_count: telemetry::present_count(), last_sample: Instant::now() }
    }
}

impl Default for FpsCollector {
    fn default() -> Self { Self::new() }
}

impl MetricCollector for FpsCollector {
    fn id(&self) -> &'static str { "fps" }
    fn label(&self) -> &'static str { "FPS" }

    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        let now = Instant::now();
        let count = telemetry::present_count();
        let elapsed = now.duration_since(self.last_sample).as_secs_f64();

        // The counter is monotonic, but guard the subtraction anyway: a wrap or a
        // reordered read must not produce a nonsense rate.
        let delta = count.saturating_sub(self.last_count);
        self.last_count = count;
        self.last_sample = now;

        // Below this floor the interval is too short to divide by meaningfully.
        // Report nothing rather than a fabricated rate.
        if elapsed < 0.05 {
            return map;
        }

        // PRESENT_COUNT is global across monitors: one render tick presents once
        // per CRTC, so on this two-monitor host the raw rate is 2x the frame rate.
        // Divide by the number of CRTCs that have actually presented, or the metric
        // reads 60 while the loop runs at 30. Caught by the Phase 1 live run, where
        // 18301 presents over ~303s across two CRTCs is 30.2 fps, not 60.4.
        let crtcs = telemetry::active_crtc_count().max(1) as f64;
        let fps = delta as f64 / elapsed / crtcs;
        map.insert(MetricId::Fps, MetricValue::String(format!("{:.1}", fps)));
        map
    }
}
