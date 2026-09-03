// src/core/telemetry/mod.rs
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

pub mod report;
pub use self::report::summary;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Total successful presents across all monitors. Read by `FpsCollector`.
static PRESENT_COUNT: AtomicU64 = AtomicU64::new(0);

/// Per-geometry X-side timings. Nanoseconds, to avoid float accumulation drift.
static PRESENT_TIMINGS: Mutex<BTreeMap<String, GeomTimings>> = Mutex::new(BTreeMap::new());

/// Per-geometry surviving `show_layout` calls: (sum, samples). Round-7 Q1 — the
/// glyph volume that actually reaches Pango, on the live path, for comparison
/// against the MRC's own survivor count.
static SURVIVED_GLYPHS: Mutex<BTreeMap<String, (u64, u64)>> = Mutex::new(BTreeMap::new());

/// Per-geometry in-process single-size control: (ns, calls). The Phase 3
/// re-entry denominator (round-7 Q3) — measured inside the running overlay, NOT
/// in cargo-test.
static LIVE_CONTROL: Mutex<BTreeMap<String, (u64, u64)>> = Mutex::new(BTreeMap::new());

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

/// Accumulated in-process cost of `RainManager::draw`, per CRTC geometry.
/// Written only when MATRIX_OVERLAY_DEBUG_METRICS is set (X-LIVE reconciliation).
static RAIN_DRAW: Mutex<BTreeMap<String, (u64, u64)>> = Mutex::new(BTreeMap::new());

/// Record one production `rain.draw` call. Nanoseconds; never logs.
pub fn record_rain_draw(w: u16, h: u16, ns: u64) {
    if let Ok(mut m) = RAIN_DRAW.lock() {
        let e = m.entry(geom_key(w, h)).or_insert((0, 0));
        e.0 = e.0.saturating_add(ns);
        e.1 = e.1.saturating_add(1);
    }
}

/// Record surviving `show_layout` calls for one production `rain.draw`.
pub fn record_survived(w: u16, h: u16, n: u32) {
    if let Ok(mut m) = SURVIVED_GLYPHS.lock() {
        let e = m.entry(geom_key(w, h)).or_insert((0, 0));
        e.0 = e.0.saturating_add(n as u64);
        e.1 = e.1.saturating_add(1);
    }
}

/// Record one in-process single-size control draw.
pub fn record_live_control(w: u16, h: u16, ns: u64) {
    if let Ok(mut m) = LIVE_CONTROL.lock() {
        let e = m.entry(geom_key(w, h)).or_insert((0, 0));
        e.0 = e.0.saturating_add(ns);
        e.1 = e.1.saturating_add(1);
    }
}

/// Snapshot of per-CRTC surviving glyph counts.
pub fn survived_snapshot() -> BTreeMap<String, (u64, u64)> {
    SURVIVED_GLYPHS.lock().map(|m| m.clone()).unwrap_or_default()
}

/// Snapshot of the in-process single-size control.
pub fn live_control_snapshot() -> BTreeMap<String, (u64, u64)> {
    LIVE_CONTROL.lock().map(|m| m.clone()).unwrap_or_default()
}

/// Snapshot of per-CRTC `rain.draw` cost.
pub fn rain_draw_snapshot() -> BTreeMap<String, (u64, u64)> {
    RAIN_DRAW.lock().map(|m| m.clone()).unwrap_or_default()
}

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

