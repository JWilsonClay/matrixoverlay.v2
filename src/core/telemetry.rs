// src/core/telemetry.rs
//! Present-path telemetry substrate (Phase 1 — S-06 and S-13b).
//!
//! Two instruments live here, both written by the presentation layer and read
//! elsewhere, which is why they sit in `core` rather than under `render`:
//! `metrics` must read the present counter without `metrics` depending on
//! `render`. `core` is the layer both already depend on (see `core::logging`).
//!
//! S-06 — PRESENT_COUNT is incremented once per successful `Presenter::present`.
//! It is deliberately NOT sourced from `Renderer::frames`, which counts `draw()`
//! calls including `Expose`-driven ones and never reaches `SharedMetrics`.
//!
//! S-13b — per-CRTC accumulators for the X-side per-frame cost. Keyed by
//! geometry because a 4096x2160x4 buffer and a 1920x1080x4 buffer are not the
//! same cost. Accumulated internally and printed ONCE at exit: a log line on the
//! path being measured is a new cost centre inside the measurement, and would
//! inflate the very number S-13b exists to establish.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Total successful presents across all monitors. Read by `FpsCollector`.
static PRESENT_COUNT: AtomicU64 = AtomicU64::new(0);

/// Per-geometry X-side timings. Nanoseconds, to avoid float accumulation drift.
static PRESENT_TIMINGS: Mutex<BTreeMap<String, GeomTimings>> = Mutex::new(BTreeMap::new());

/// Accumulated X-side cost for one CRTC geometry.
#[derive(Debug, Default, Clone, Copy)]
pub struct GeomTimings {
    pub pre_draw_ns: u64,
    pub put_image_ns: u64,
    pub gc_ns: u64,
    pub presents: u64,
    pub pre_draws: u64,
}

/// A restartable stopwatch. `lap()` returns nanoseconds since the last lap and
/// resets, so a sequence of calls partitions a block with no overlap or gap.
pub struct Lap(Instant);

impl Lap {
    pub fn new() -> Self { Self(Instant::now()) }

    pub fn lap(&mut self) -> u64 {
        let now = Instant::now();
        let ns = now.duration_since(self.0).as_nanos() as u64;
        self.0 = now;
        ns
    }
}

impl Default for Lap {
    fn default() -> Self { Self::new() }
}

fn geom_key(w: u16, h: u16) -> String { format!("{}x{}", w, h) }

/// Record a `pre_draw` round-trip for one CRTC. Never logs.
pub fn record_pre_draw(w: u16, h: u16, ns: u64) {
    if let Ok(mut map) = PRESENT_TIMINGS.lock() {
        let e = map.entry(geom_key(w, h)).or_default();
        e.pre_draw_ns = e.pre_draw_ns.saturating_add(ns);
        e.pre_draws = e.pre_draws.saturating_add(1);
    }
}

/// Record one successful present for one CRTC, and bump the global S-06 counter.
/// `gc_ns` covers CreateGc plus FreeGc/flush; `put_image_ns` covers the image put.
pub fn record_present(w: u16, h: u16, put_image_ns: u64, gc_ns: u64) {
    PRESENT_COUNT.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut map) = PRESENT_TIMINGS.lock() {
        let e = map.entry(geom_key(w, h)).or_default();
        e.put_image_ns = e.put_image_ns.saturating_add(put_image_ns);
        e.gc_ns = e.gc_ns.saturating_add(gc_ns);
        e.presents = e.presents.saturating_add(1);
    }
}

/// Monotonic count of successful presents. Source of truth for the `fps` metric.
pub fn present_count() -> u64 { PRESENT_COUNT.load(Ordering::Relaxed) }

/// Number of distinct CRTC geometries that have presented at least once.
///
/// One render tick presents once per monitor, so `PRESENT_COUNT` is the frame
/// rate multiplied by this. `FpsCollector` divides by it to report the loop's
/// actual frame rate rather than a per-monitor total.
pub fn active_crtc_count() -> usize {
    PRESENT_TIMINGS.lock().map(|m| m.len()).unwrap_or(0)
}

/// Snapshot of the per-CRTC accumulators. Used by the exit summary and by tests.
pub fn timings_snapshot() -> BTreeMap<String, GeomTimings> {
    PRESENT_TIMINGS.lock().map(|m| m.clone()).unwrap_or_default()
}

fn mean_ms(total_ns: u64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { total_ns as f64 / n as f64 / 1_000_000.0 }
}

/// Render the S-13b summary. Printed exactly once, at shutdown.
pub fn summary() -> String {
    let snap = timings_snapshot();
    let mut out = String::from(
        "\n=== S-13b — X-side per-frame cost, per CRTC (means, ms) ===\n\
         geometry        presents   pre_draw   put_image         gc       total\n",
    );
    if snap.is_empty() {
        out.push_str("  (no presents recorded)\n");
        return out;
    }
    let mut grand = 0.0;
    for (geom, t) in &snap {
        let pre = mean_ms(t.pre_draw_ns, t.pre_draws);
        let put = mean_ms(t.put_image_ns, t.presents);
        let gc = mean_ms(t.gc_ns, t.presents);
        let total = pre + put + gc;
        grand += total;
        out.push_str(&format!(
            "{:<15} {:>8} {:>10.4} {:>11.4} {:>10.4} {:>11.4}\n",
            geom, t.presents, pre, put, gc, total
        ));
    }
    out.push_str(&format!(
        "present_ms summed across CRTCs: {:.4} ms/frame   (total presents: {})\n",
        grand,
        present_count()
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [S-13b] mean_ms converts accumulated nanoseconds to a per-event mean in ms,
    /// and must not divide by zero when a slot recorded nothing.
    #[test]
    fn mean_ms_converts_and_guards_zero() {
        assert_eq!(mean_ms(0, 0), 0.0);
        assert!((mean_ms(1_000_000, 1) - 1.0).abs() < 1e-9);
        // The live Phase 1 reading: 4096x2160 pre_draw averaged 1.1962 ms.
        assert!((mean_ms(1_196_200 * 100, 100) - 1.1962).abs() < 1e-6);
    }

    /// [S-06] One render tick presents once per CRTC, so the raw counter runs at
    /// `fps * crtcs`. This is the arithmetic `FpsCollector` applies; the live run
    /// produced 18301 presents over ~303 s across 2 CRTCs = 30.2 fps, not 60.4.
    #[test]
    fn present_count_is_per_crtc_not_per_frame() {
        let raw_rate: f64 = 18301.0 / 303.0;
        assert!((raw_rate - 60.4).abs() < 0.2, "raw rate {}", raw_rate);
        let frame_rate: f64 = raw_rate / 2.0;
        assert!((frame_rate - 30.2).abs() < 0.2, "frame rate {}", frame_rate);
    }
}
