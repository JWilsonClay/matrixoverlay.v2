//! Phase 5 — frame governor (S-07 / F4) and `target_fps` config contract.
//!
//! # R-06 applies here too
//!
//! These tests call the **production** timing functions
//! (`core::threads::tick_period`, `core::threads::next_deadline`) and the
//! **production** clamp (`General::fps`). The governor's rule is deliberately
//! factored into pure functions so it can be tested without spawning a thread
//! and sleeping — a test that slept would measure the OS scheduler, not the
//! rule, and would be flaky on a loaded machine for reasons unrelated to F4.

use std::time::{Duration, Instant};

use matrix_overlay::core::config::Config;
use matrix_overlay::core::threads::{next_deadline, tick_period};

/// **S-07 / AC1 — the F4 fix.** Drive the governor through a run of frames that
/// each overrun their period, and assert the **achieved rate** stays bounded.
///
/// # Why this asserts a rate and not a single step
///
/// The first version of this test checked one step: that the next deadline fell
/// after the slow frame ended and within one period of it. It **passed against a
/// deliberately fail-open implementation** (`next_deadline` returning
/// `now + 1ms`), because a 1 ms tick also satisfies both of those. That is a
/// Mock Trap forming inside the test written to prevent F4, and it was caught by
/// temporarily reinstating the broken behavior — the same red-before-green check
/// this campaign requires of every performance assertion.
///
/// F4's signature is not one early tick; it is an **unbounded issue rate under
/// load**. The old loop measured `elapsed` across a blocking `send()` on a
/// `bounded(1)` channel, so a late renderer made `elapsed` exceed the interval,
/// the thread slept **1 ms**, and it re-queued immediately — running fastest
/// exactly when it was already too slow. Only a rate assertion can see that.
#[test]
fn governor_does_not_fail_open_on_slow_frames() {
    const FRAMES: u32 = 20;
    for fps in [1u32, 5, 30] {
        let period = tick_period(fps);
        // Every frame overruns the 5 and 30 fps periods.
        let frame_cost = Duration::from_millis(200);

        let start = Instant::now();
        let mut deadline = start + period;
        let mut now = start;
        for _ in 0..FRAMES {
            now += frame_cost;                                  // the frame runs long
            if now < deadline { now = deadline; }               // the loop sleeps to the deadline
            deadline = next_deadline(deadline, now, period);
        }

        let elapsed = now - start;
        let floor = period * (FRAMES - 1);
        assert!(
            elapsed >= floor,
            "fps={fps}: {FRAMES} frames took {elapsed:?}, under the {floor:?} the rate allows — \
             the governor failed open"
        );
        // And it must not have queued catch-up frames to "make up" the overruns:
        // the run can exceed the floor only by the per-frame overrun itself.
        let ceiling = floor + frame_cost * FRAMES;
        assert!(elapsed <= ceiling, "fps={fps}: {elapsed:?} exceeds {ceiling:?} — drift, not skip");
    }
}

/// A single overrun must not pull the next tick in ahead of schedule.
#[test]
fn governor_skips_missed_ticks_rather_than_queueing_them() {
    for fps in [1u32, 5, 30] {
        let period = tick_period(fps);
        let start = Instant::now();
        let deadline = start + period;
        let stalled = start + Duration::from_secs(10);
        let next = next_deadline(deadline, stalled, period);
        assert!(next > stalled, "fps={fps}: tick issued at or before the stall ended");
        assert!(
            next - stalled <= period,
            "fps={fps}: recovery drifted by more than one period ({:?})", next - stalled
        );
    }
}

/// The period is the reciprocal of the rate, and 1 fps is a full second — the
/// default the live budget identity forced (20.64 ms/tick x 1 / 10 = 2.06%).
#[test]
fn tick_period_matches_the_configured_rate() {
    assert_eq!(tick_period(1), Duration::from_secs(1));
    assert_eq!(tick_period(30), Duration::from_nanos(33_333_333));
    assert_eq!(tick_period(60), Duration::from_nanos(16_666_666));
}

/// **AC3 — the clamp.** Zero must not divide; an absurd value must not recreate
/// the runaway. Asserted on the production accessor AND on `tick_period`, since
/// either could be reached first.
#[test]
fn target_fps_is_clamped_both_ways() {
    let mut c = Config::default();

    c.general.target_fps = 0;
    assert_eq!(c.general.fps(), 1, "zero must clamp to 1, never divide");
    assert_eq!(tick_period(0), Duration::from_secs(1));

    c.general.target_fps = 9999;
    assert_eq!(c.general.fps(), 60, "absurd values must clamp to 60");
    assert_eq!(tick_period(9999), Duration::from_nanos(16_666_666));

    c.general.target_fps = 5;
    assert_eq!(c.general.fps(), 5, "in-band values must pass through unchanged");
}

/// **AC4 / C-02.** A `config.json` written before `target_fps` existed must
/// still parse under `#[serde(deny_unknown_fields)]` and default to **1**.
///
/// The JSON below is a minimal real-shaped config with no `target_fps` key. If
/// this test ever fails, every existing user config on disk has been broken.
#[test]
fn config_without_target_fps_loads_and_defaults_to_one() {
    let json = r##"{
        "general": {
            "font_size": 16, "color": "#00FF41", "update_ms": 2000
        },
        "screens": [{ "metrics": ["cpu_usage"], "x_offset": 20, "y_offset": 20 }],
        "weather": { "lat": 0.0, "lon": 0.0, "enabled": false, "auto_location": false }
    }"##;

    let c: Config = serde_json::from_str(json).expect("pre-target_fps config must still parse");
    assert_eq!(c.general.target_fps, 1, "missing target_fps must default to 1, not 10");
    assert_eq!(c.general.fps(), 1);
}

/// The default is **1**, not the pre-audit placeholder 10. Guards the number the
/// whole S-04 projection rests on.
#[test]
fn default_target_fps_is_one() {
    assert_eq!(Config::default().general.target_fps, 1);
}
