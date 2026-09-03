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

/// Config matching the live substrate at the time of the 2026-09-03 investigation.
fn mrc_config() -> Config {
    let mut c = Config::default();
    c.general.font_size = MRC_FONT_SIZE;
    c.cosmetics.realism = MRC_REALISM;
    c.cosmetics.rain_mode = "fall".to_string();
    c
}

/// Build a `RainManager` and run it to a steady-state distribution.
///
/// `reset()` seeds every stream above the viewport (`y` in `-h..0`). Measuring
/// from that state would time a screen with most glyphs clipped out by the
/// `y < -20.0 || y > h + 20.0` guard in `draw`, understating the real cost. The
/// priming loop below advances the simulation until streams are spread across
/// the full height, which is what production renders.
fn primed_manager(config: &Config) -> RainManager {
    let mut rain = RainManager::new(MRC_REALISM);
    let dt = std::time::Duration::from_millis(33);
    for _ in 0..600 {
        rain.update(dt, MRC_W, MRC_H, config);
    }
    rain
}

/// Render `MRC_FRAMES` frames through production `draw` and return per-frame ms.
fn measure_frames(rain: &RainManager, config: &Config) -> Vec<f64> {
    let surface = ImageSurface::create(Format::ARgb32, MRC_W, MRC_H).unwrap();
    let mut series = Vec::with_capacity(MRC_FRAMES);
    for frame in 0..MRC_FRAMES {
        // Match production's per-frame setup exactly (pipeline.rs::draw): a FRESH
        // Cairo Context each frame, and an opaque clear before the rain draw.
        // Reusing one Context across frames and never clearing — as this harness
        // first did — measured 610 ms/frame against production's 3.95 ms, a 155x
        // divergence. That is the X-LIVE failure mode: a test faithfully timing a
        // path production does not take.
        let cr = Context::new(&surface).unwrap();
        cr.set_operator(cairo::Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.paint().unwrap();
        cr.set_operator(cairo::Operator::Over);

        let t = Instant::now();
        rain.draw(&cr, MRC_W as f64, MRC_H as f64, frame as u64, config)
            .expect("production RainManager::draw failed");
        series.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    series
}

fn mean(v: &[f64]) -> f64 { v.iter().sum::<f64>() / v.len() as f64 }

fn report(name: &str, series: &[f64], streams: usize, distinct_sizes: usize) {
    let profile = if cfg!(debug_assertions) { "dev" } else { "release" };
    let mut sorted = series.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "\n[{name}] profile={profile} geometry={MRC_W}x{MRC_H} realism={MRC_REALISM} \
         font_size={MRC_FONT_SIZE} streams={streams} distinct_sizes_per_frame={distinct_sizes}"
    );
    println!(
        "[{name}] mean={:.3} ms  p50={:.3}  p95={:.3}  frame1={:.3}  frame{}={:.3}",
        mean(series), sorted[series.len() / 2], sorted[(series.len() * 95) / 100],
        series[0], series.len(), series[series.len() - 1]
    );
    println!("[{name}] series={:?}", series.iter().map(|x| (x * 1000.0).round() / 1000.0).collect::<Vec<_>>());
}

/// **THE MRC — a validation, not a control.** Calls production
/// `RainManager::draw` with production-shaped inputs: 4096x2160, realism=4,
/// font_size=16, streams primed to steady state, 40 consecutive frames.
///
/// Gate: mean **< 20 ms/frame** (S-01). This is expected to FAIL until Phase 3.
/// A pass here before Phase 3 means the test is not exercising the production
/// path — fix the test, never the threshold.
#[test]
fn test_rain_frame_cost_mrc() {
    let config = mrc_config();
    let rain = primed_manager(&config);
    let distinct = {
        let mut d: Vec<u64> = rain.streams.iter()
            .map(|s| ((config.general.font_size as f64 * 0.8 * s.depth) * 1024.0) as u64)
            .collect();
        d.sort_unstable(); d.dedup(); d.len()
    };
    let series = measure_frames(&rain, &config);
    report("MRC", &series, rain.streams.len(), distinct);

    let m = mean(&series);
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
#[test]
fn test_rain_fixed_size_control() {
    let config = mrc_config();
    let mut rain = primed_manager(&config);
    for s in &mut rain.streams { s.depth = 1.0; }
    let series = measure_frames(&rain, &config);
    report("CONTROL", &series, rain.streams.len(), 1);
    println!("[CONTROL] label: control, not a validation - single font size is a path production never takes");
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
    let mut rain = primed_manager(&config);
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
