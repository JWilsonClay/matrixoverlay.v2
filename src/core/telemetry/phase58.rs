// src/core/telemetry/phase58.rs
//! [Phase 5.8] Residual-isolation instruments.
//!
//! Round 9's ruling: the 1.073% left over after the render subtotal is a
//! *subtraction*, not a floor. Two rate-dependent terms are hiding in it —
//! `clear()` and the metrics glow — and only what remains after those are named
//! is the true frame-rate-independent floor. These accumulators name them.
//!
//! Same discipline as S-13b: accumulate internally, print once at exit. A log
//! line on the path being measured is a new cost centre inside the measurement.
//!
//! Lives in its own file rather than in `mod.rs` because `mod.rs` is at 157
//! lines and C-01's hard cap is 175.

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::geom_key;

/// [F-B] Per-geometry `Renderer::clear()` cost. Separate from `PRESENT_TIMINGS`
/// because this is Cairo-side, not X-side, and the budget identity scales the
/// two differently.
static CLEAR_NS: Mutex<BTreeMap<String, (u64, u64)>> = Mutex::new(BTreeMap::new());

/// [F-A] Per-geometry `draw_metrics` cost — the six-`show_layout`-per-metric
/// glow path. Never measured before this phase.
static GLOW_NS: Mutex<BTreeMap<String, (u64, u64)>> = Mutex::new(BTreeMap::new());

/// [F-C] Collector-thread totals: (tick ns, ticks, nvidia-smi ns, nvidia calls).
static COLLECTORS: Mutex<(u64, u64, u64, u64)> = Mutex::new((0, 0, 0, 0));

/// [F-C] `nvidia-smi` subprocess time, accumulated at the call site and drained
/// once per collector cycle.
static NVIDIA: Mutex<(u64, u64)> = Mutex::new((0, 0));

fn bump(m: &Mutex<BTreeMap<String, (u64, u64)>>, w: u16, h: u16, ns: u64) {
    if let Ok(mut m) = m.lock() {
        let e = m.entry(geom_key(w, h)).or_insert((0, 0));
        e.0 = e.0.saturating_add(ns);
        e.1 = e.1.saturating_add(1);
    }
}

pub fn record_clear(w: u16, h: u16, ns: u64) { bump(&CLEAR_NS, w, h, ns); }
pub fn record_glow(w: u16, h: u16, ns: u64) { bump(&GLOW_NS, w, h, ns); }

/// One metrics-collector cycle, with the `nvidia-smi` share broken out —
/// `CLAUDE.md` warns that polling it wakes the sleeping dGPU, which makes it the
/// leading suspect for the floor.
pub fn record_collector_tick(tick_ns: u64, nvidia_ns: u64, nvidia_calls: u64) {
    if let Ok(mut c) = COLLECTORS.lock() {
        c.0 = c.0.saturating_add(tick_ns);
        c.1 = c.1.saturating_add(1);
        c.2 = c.2.saturating_add(nvidia_ns);
        c.3 = c.3.saturating_add(nvidia_calls);
    }
}

pub fn add_nvidia_call(ns: u64) {
    if let Ok(mut n) = NVIDIA.lock() { n.0 = n.0.saturating_add(ns); n.1 += 1; }
}

/// Drain the `nvidia-smi` accumulator. Called once per collector cycle so the
/// subprocess time is attributed to the cycle that spent it.
pub fn take_nvidia() -> (u64, u64) {
    NVIDIA.lock().map(|mut n| { let v = *n; *n = (0, 0); v }).unwrap_or((0, 0))
}

pub fn clear_snapshot() -> BTreeMap<String, (u64, u64)> {
    CLEAR_NS.lock().map(|m| m.clone()).unwrap_or_default()
}
pub fn glow_snapshot() -> BTreeMap<String, (u64, u64)> {
    GLOW_NS.lock().map(|m| m.clone()).unwrap_or_default()
}
pub fn collector_snapshot() -> (u64, u64, u64, u64) {
    COLLECTORS.lock().map(|c| *c).unwrap_or((0, 0, 0, 0))
}
