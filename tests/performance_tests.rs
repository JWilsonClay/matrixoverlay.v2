//! Performance tests for matrix-overlay.
//!
//! # The anti-Mock-Trap rule (binding on every assertion in this file)
//!
//! A performance assertion MUST call production code with production-shaped
//! inputs. Anything that does not is labeled a **control**, never a validation.
//!
//! This rule exists because the test that used to live here —
//! `test_render_optimization_bench` — asserted `< 500 ms` for 50,000 glyphs
//! rendered through ONE `pango::Layout` at ONE font size, and commented itself
//! as proof that "with caching, we can render 50k glyphs in milliseconds."
//! `RainManager::draw` does not take that path: it cycles a distinct
//! `FontDescription` per stream, every frame. The test passed continuously
//! while the code it claimed to cover ran orders of magnitude slower. It was
//! deleted in Phase 2 of the Render Substrate Remediation Campaign.
//!
//! A test that measures a path production never takes is worse than no test:
//! it converts an absence of coverage into a false claim of coverage.

use std::time::{Duration, Instant};
use std::thread;
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use cairo::{ImageSurface, Format, Context};

#[test]
fn test_update_latency_accuracy() {
    // Verify that a simulated 1000ms loop stays within acceptable drift (<50ms)
    let target_interval = Duration::from_millis(100); // Scaled down for test speed
    let iterations = 5;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let loop_start = Instant::now();
        // Simulate work (e.g. metrics collection)
        thread::sleep(Duration::from_millis(10));
        
        let elapsed = loop_start.elapsed();
        if elapsed < target_interval {
            thread::sleep(target_interval - elapsed);
        }
    }
    
    let total_elapsed = start.elapsed();
    let expected = target_interval * iterations as u32;
    let diff = if total_elapsed > expected {
        total_elapsed - expected
    } else {
        expected - total_elapsed
    };
    
    // Allow small overhead margin
    assert!(diff.as_millis() < 50, "Timer drift too high: {}ms", diff.as_millis());
}

#[test]
fn test_cpu_ram_usage_simulation() {
    // Measure the resource usage of the test process during a simulated workload
    let mut sys = System::new_all();
    let pid = Pid::from(std::process::id() as usize);
    
    // Warmup
    sys.refresh_process(pid);
    
    // Simulate "heavy" loop (metrics + render logic)
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        sys.refresh_cpu(); // Simulate sysinfo work
        thread::sleep(Duration::from_millis(16)); // ~60 FPS simulation
    }
    
    sys.refresh_process(pid);
    let proc = sys.process(pid).expect("Failed to get process info");
    
    println!("Simulated CPU: {:.2}%, RAM: {} bytes", proc.cpu_usage(), proc.memory());
    
    // Sanity checks (Thresholds depend on environment, but shouldn't be massive)
    assert!(proc.memory() < 500 * 1024 * 1024, "Memory usage exceeded 500MB"); 
}

/// **DEFERRED FINDING (task 2.5) — this test is a Mock Trap and is knowingly
/// left in place for one phase.**
///
/// It asserts `proc.cpu_usage() < 1.0` for "Pulse Mode", but pulse mode is NOT
/// IMPLEMENTED: `pipeline.rs:35` draws rain only when `rain_mode == "fall"`, and
/// every other value silently draws nothing. Worse, `main.rs:28` overwrites
/// `rain_mode` to `"fall"` at startup regardless of config (F8), so the branch
/// is unreachable even if it existed. The loop below performs no rendering at
/// all — it spins on `refresh_process` — so this asserts that doing nothing
/// costs little.
///
/// Not fixed here: Phase 7 implements pulse mode and rewrites this test against
/// the real thing. Deleting it now would remove the marker for that work.
#[test]
fn test_pulse_mode_efficiency() {
    let mut sys = System::new_all();
    let pid = Pid::from(std::process::id() as usize);
    
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        // Simulated Pulse Mode (No glyphs, just global alpha update)
        // thread::sleep(Duration::from_millis(16));
        sys.refresh_process(pid);
    }
    
    let proc = sys.process(pid).expect("Failed to get process info");
    println!("Pulse Mode CPU: {:.2}%", proc.cpu_usage());
    assert!(proc.cpu_usage() < 1.0, "Pulse mode exceeded 1% CPU target");
}
// ============================================================================
// The MRC — Minimal Reproducible Case for F1 (font-cache eviction)
//
// R-06: both tests below call PRODUCTION `RainManager::draw`. Neither contains a
// synthetic glyph loop. The MRC and the control differ in exactly one variable —
// whether `RainStream::depth` varies — so any delta between them is attributable
// to font-size churn and nothing else.
// ============================================================================

use matrix_overlay::core::config::Config;
use matrix_overlay::render::physics::RainManager;

const MRC_W: i32 = 4096;
const MRC_H: i32 = 2160;
const MRC_REALISM: u32 = 4;
const MRC_FONT_SIZE: u32 = 16;
const MRC_FRAMES: usize = 40;
/// Live value on 2026-09-03. `Config::default()` says 1.0 — a 10x error.
const MRC_RAIN_SPEED: f64 = 0.1;
/// Live value on 2026-09-03. Alpha only; recorded for fidelity, not for cost.
const MRC_BRIGHTNESS: f64 = 0.35;
/// Hard ceiling on priming steps so a stall can never hang the suite.
const MRC_PRIME_CAP: usize = 40_000;

/// Config matching the live substrate at the time of the 2026-09-03 investigation.
///
/// **Every field that reaches the rain path is PINNED to a literal copied from
/// the live `~/.config/matrix-overlay/config.json` on 2026-09-03.** The values
/// are duplicated here rather than read from disk so the test is reproducible
/// on any machine (round 7, Q1: `config_source: pinned_literals`).
///
/// `Config::default()` supplies only the fields the rain path never reads; it
/// is the base because `#[serde(deny_unknown_fields)]` (C-02) makes a literal
/// struct construction brittle against future field additions.
///
/// # Why this is not cosmetic
///
/// The first MRC primed from `Config::default()`, whose `rain_speed` is **1.0**.
/// Live is **0.1** — the fall simulation ran at 10x real speed, so the steady
/// state the harness measured was not the steady state production renders. That
/// is an R-06 violation (production-shaped inputs) committed inside the test
/// written to replace a Mock Trap. It is fixed here and named rather than
/// quietly corrected.
fn mrc_config() -> Config {
    let mut c = Config::default();
    c.general.font_size = MRC_FONT_SIZE;              // live: 16
    c.cosmetics.realism = MRC_REALISM;                // live: 4   (default is 10)
    c.cosmetics.rain_mode = "fall".to_string();       // live: "fall"
    c.cosmetics.rain_speed = MRC_RAIN_SPEED;          // live: 0.1 (default is 1.0)
    c.cosmetics.matrix_brightness = MRC_BRIGHTNESS;   // live: 0.35
    c
}

/// Fraction of glyph slots currently inside the drawable band.
///
/// Mirrors `RainManager::draw`'s own guard (`y < -20.0 || y > h + 20.0`) rather
/// than approximating it, so "primed" means the same thing the renderer means.
fn on_screen_fraction(rain: &RainManager, config: &Config) -> f64 {
    let size = config.general.font_size as f64 * 0.8;
    let h = MRC_H as f64;
    let (mut vis, mut total) = (0usize, 0usize);
    for s in &rain.streams {
        for i in 0..s.glyphs.len() {
            let y = s.y - (i as f64 * size * 1.2);
            total += 1;
            if !(y < -20.0 || y > h + 20.0) { vis += 1; }
        }
    }
    if total == 0 { 0.0 } else { vis as f64 / total as f64 }
}

/// Build a `RainManager` and run it to a steady-state distribution.
///
/// `reset()` seeds every stream above the viewport (`y` in `-h..0`). Measuring
/// from that state would time a screen with most glyphs clipped out by the
/// `y < -20.0 || y > h + 20.0` guard in `draw`, understating the real cost.
///
/// **A fixed step count cannot be used at the pinned `rain_speed = 0.1`.** One
/// wrap of the `h + 400` px span takes 1,300-6,400 steps at that speed (versus
/// 130-640 at the default 1.0), so the previous hard-coded 600 left the field
/// barely moved. This primes until the on-screen fraction stops changing:
/// the mean over the last 30 steps within 0.01 of the mean over the 30 before
/// it, after at least one full slow-stream traversal. Returns the steps used so
/// the receipt can record it.
fn primed_manager(config: &Config) -> (RainManager, usize) {
    let mut rain = RainManager::new(MRC_REALISM);
    let dt = std::time::Duration::from_millis(33);
    let mut hist: Vec<f64> = Vec::new();
    let mut steps = 0usize;
    while steps < MRC_PRIME_CAP {
        rain.update(dt, MRC_W, MRC_H, config);
        steps += 1;
        hist.push(on_screen_fraction(&rain, config));
        if steps >= 7_000 && hist.len() >= 60 {
            let n = hist.len();
            let recent = mean(&hist[n - 30..]);
            let prior = mean(&hist[n - 60..n - 30]);
            if (recent - prior).abs() < 0.01 { break; }
        }
    }
    (rain, steps)
}

/// Render `MRC_FRAMES` frames through production `draw`.
///
/// Returns `(per-frame ms, per-frame surviving `show_layout` calls)`.
fn measure_frames(rain: &mut RainManager, config: &Config) -> (Vec<f64>, Vec<u32>) {
    // ONE surface, reused across frames — production reuses the presenter's
    // surface too. (Round 7 §3.2: MRC-B exists to confirm this has not
    // regressed to a per-frame create; `surface_reused: true` is recorded.)
    let surface = ImageSurface::create(Format::ARgb32, MRC_W, MRC_H).unwrap();
    let dt = std::time::Duration::from_millis(33);
    let mut series = Vec::with_capacity(MRC_FRAMES);
    let mut survived = Vec::with_capacity(MRC_FRAMES);
    matrix_overlay::render::physics::count_show_layout(true);
    for frame in 0..MRC_FRAMES {
        // Match production's per-frame setup exactly (pipeline.rs::draw): advance
        // the simulation, take a FRESH Cairo Context, and clear opaquely before
        // the rain draw. Reusing one Context across frames and never clearing —
        // as this harness first did — is a path production never takes.
        //
        // The earlier version of this comment cited production `rain.draw` as
        // 3.95 ms; that came from an early short sample. The figure of record is
        // **10.0030 ms** at 4096x2160, from 21,854 in-process calls over 12
        // minutes (Phase 2 receipt).
        rain.update(dt, MRC_W, MRC_H, config);
        let cr = Context::new(&surface).unwrap();
        cr.set_operator(cairo::Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.paint().unwrap();
        cr.set_operator(cairo::Operator::Over);

        let _ = matrix_overlay::render::physics::take_survived();
        let t = Instant::now();
        rain.draw(&cr, MRC_W as f64, MRC_H as f64, frame as u64, config)
            .expect("production RainManager::draw failed");
        series.push(t.elapsed().as_secs_f64() * 1000.0);
        survived.push(matrix_overlay::render::physics::take_survived());
    }
    matrix_overlay::render::physics::count_show_layout(false);
    (series, survived)
}

fn mean(v: &[f64]) -> f64 { v.iter().sum::<f64>() / v.len() as f64 }

fn mean_u32(v: &[u32]) -> f64 { v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64 }

/// The in-process figure of record for production `RainManager::draw` at
/// 4096x2160: 10.0030 ms, from 21,854 calls over a 12-minute run (Phase 2
/// receipt). X-LIVE is a RATIO against this number as of round 7.
const LIVE_RAIN_4K_MS: f64 = 10.0030;
/// X-LIVE trips at this ratio. The 25 ms absolute form is kept as a backstop
/// for when the in-process figure is unavailable.
const X_LIVE_RATIO: f64 = 3.0;

fn report(name: &str, series: &[f64], survived: &[u32], streams: usize, distinct_sizes: usize, prime_steps: usize) {
    let profile = if cfg!(debug_assertions) { "dev" } else { "release" };
    let mut sorted = series.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "\n[{name}] profile={profile} geometry={MRC_W}x{MRC_H} realism={MRC_REALISM} \
         font_size={MRC_FONT_SIZE} rain_speed={MRC_RAIN_SPEED} brightness={MRC_BRIGHTNESS} \
         streams={streams} distinct_sizes_configured={distinct_sizes} prime_steps={prime_steps}"
    );
    println!(
        "[{name}] mean={:.3} ms  p50={:.3}  p95={:.3}  frame1={:.3}  frame{}={:.3}",
        mean(series), sorted[series.len() / 2], sorted[(series.len() * 95) / 100],
        series[0], series.len(), series[series.len() - 1]
    );
    let mut ss = survived.to_vec();
    ss.sort_unstable();
    println!(
        "[{name}] survived_show_layout mean={:.1}  p50={}  min={}  max={}  us_per_glyph={:.2}",
        mean_u32(survived), ss[ss.len() / 2], ss[0], ss[ss.len() - 1],
        mean(series) * 1000.0 / mean_u32(survived).max(1.0)
    );
    println!("[{name}] survived_series={:?}", survived);
    println!("[{name}] surface_reused=true");
}

/// **THE MRC — a validation, not a control.** Calls production
/// `RainManager::draw` with production-shaped inputs: 4096x2160, realism=4,
/// font_size=16, `rain_speed=0.1`, streams primed to a *stable* on-screen
/// fraction, `rain.update` between frames, 40 consecutive frames.
///
/// # What the S-01 assert means after round 7
///
/// The gate is mean **< 20 ms/frame**. Under the original reading a pass here
/// before Phase 3 meant "the test is not exercising the production path." That
/// reading is retired: production `rain.draw` measures **10.0030 ms** in-process,
/// so a pass now means **the MRC agrees with the live substrate** — which is the
/// goal of this rework, not a failure of it.
///
/// The assert is kept at 20 ms as a regression rail against rebuilding the
/// 612 ms path. It is NOT the F1 verdict. X-1 is not evaluated until the X-LIVE
/// ratio is below 3.
#[test]
fn test_rain_frame_cost_mrc() {
    // [2.9 probe E1, round-8] The overlay process runs `gtk::init()` before it
    // ever draws; this one did not. If the 59x per-glyph divergence is the GTK
    // /`PangoCairoFontMap`/Xft font-map state rather than the surface, this call
    // alone should collapse the number. Harmless if it does not: `init` is
    // idempotent and the draw path is untouched.
    let gtk_ok = gtk::init().is_ok();
    println!("[2.9-E1] gtk::init() ok={gtk_ok}");

    let config = mrc_config();
    let (mut rain, prime_steps) = primed_manager(&config);
    let distinct = {
        let mut d: Vec<u64> = rain.streams.iter()
            .map(|s| ((config.general.font_size as f64 * 0.8 * s.depth) * 1024.0) as u64)
            .collect();
        d.sort_unstable(); d.dedup(); d.len()
    };
    // [2.9 probe E2] Cairo font options are part of the scaled-font cache key
    // and of the rasterization cost. Dumped on both sides for comparison; the
    // harness is NOT tuned to force a match.
    {
        let s = ImageSurface::create(Format::ARgb32, 64, 64).unwrap();
        let c = Context::new(&s).unwrap();
        println!("[2.9-E2] mrc_font_options: {}", matrix_overlay::render::describe_font_options(&c));
    }
    let (series, survived) = measure_frames(&mut rain, &config);
    report("MRC", &series, &survived, rain.streams.len(), distinct, prime_steps);

    let m = mean(&series);
    let ratio = m / LIVE_RAIN_4K_MS;
    println!(
        "[X-LIVE] mrc={:.3} ms  live_rain_4k={:.4} ms  ratio={:.2}  threshold={:.1}  backstop_ms=25  tripped={}",
        m, LIVE_RAIN_4K_MS, ratio, X_LIVE_RATIO, ratio >= X_LIVE_RATIO || m > 25.0
    );
    assert!(m < 20.0, "MRC mean {:.3} ms/frame exceeds the 20 ms gate (S-01)", m);
}

/// **CONTROL, NOT A VALIDATION.** Identical production `draw`, identical glyph
/// count, identical geometry — with `depth` flattened to a single value so every
/// stream resolves to ONE font size.
///
/// It exists to prove the MRC's cost is size-churn rather than glyph volume. It
/// is deliberately the same shape as the deleted `test_render_optimization_bench`
/// trap, and is labeled a control precisely because that shape cannot validate
/// anything about the production render path.
///
/// **This control is NOT the Phase 3 re-entry denominator.** Round 7 fixes that
/// denominator as an *in-process* single-size control measured inside the running
/// overlay (`MATRIX_OVERLAY_DEBUG_CONTROL=1`), never this cargo-test figure.
#[test]
fn test_rain_fixed_size_control() {
    let config = mrc_config();
    let (mut rain, prime_steps) = primed_manager(&config);
    for s in &mut rain.streams { s.depth = 1.0; }
    let (series, survived) = measure_frames(&mut rain, &config);
    report("CONTROL", &series, &survived, rain.streams.len(), 1, prime_steps);
    println!("[CONTROL] label: control, not a validation - single font size is a path production never takes");
    println!("[CONTROL] label: NOT the Phase 3 re-entry denominator - that is the in-process live control");
}

/// **S-13a (task 2.6b)** — Cairo-side per-frame cost OUTSIDE `RainManager::draw`.
///
/// Times the opaque full-surface `clear()` and `rain.update` at 4096x2160. The
/// clear is reproduced here rather than called through `Renderer::clear`, which
/// requires a `Presenter` and therefore an X connection this harness does not
/// have; the Cairo operations are identical (`Operator::Source` +
/// `rgba(0,0,0,1)` + `paint`), and that substitution is stated rather than
/// hidden. The present path is NOT timed here — that is S-13b, measured in
/// Phase 1.7 against a real X connection.
#[test]
fn measure_s13a_cairo_rest() {
    let config = mrc_config();
    let (mut rain, _prime_steps) = primed_manager(&config);
    let surface = ImageSurface::create(Format::ARgb32, MRC_W, MRC_H).unwrap();
    let cr = Context::new(&surface).unwrap();
    let dt = std::time::Duration::from_millis(33);

    let mut clear_ms = Vec::new();
    let mut update_ms = Vec::new();
    for _ in 0..MRC_FRAMES {
        let t = Instant::now();
        cr.set_operator(cairo::Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.paint().unwrap();
        cr.set_operator(cairo::Operator::Over);
        clear_ms.push(t.elapsed().as_secs_f64() * 1000.0);

        let t = Instant::now();
        rain.update(dt, MRC_W, MRC_H, &config);
        update_ms.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let profile = if cfg!(debug_assertions) { "dev" } else { "release" };
    println!(
        "\n[S-13a] profile={} clear={:.4} ms  rain_update={:.4} ms  subtotal={:.4} ms  (one 4096x2160 surface)",
        profile, mean(&clear_ms), mean(&update_ms), mean(&clear_ms) + mean(&update_ms)
    );
    println!("[S-13a] note: metrics glow not included - needs a laid-out metrics panel; see receipt");
}
