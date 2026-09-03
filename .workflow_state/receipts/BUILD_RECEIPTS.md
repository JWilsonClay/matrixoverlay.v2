## 2026-09-03 — /execute-build — Phase 1: Instrumentation Truth
- Phase/Stage: Phase 1: Instrumentation Truth
- Grade/Status: TASKS COMPLETE — AC1/AC2/AC3 UNMET (phase not certified)
- Files: src/core/telemetry.rs, src/metrics/collectors/system/fps.rs | src/core/mod.rs, src/core/main.rs, src/metrics/mod.rs, src/metrics/dispatch.rs, src/metrics/collectors/system/mod.rs, src/metrics/collectors/system/process.rs, src/render/engine/presentation/shm.rs, src/render/engine/presentation/socket.rs, tasks.md
- Deviation Log: NONE — no mid-phase HALT/approval exchange occurred. Two in-scope additions are recorded in the notes below (SIGTERM handler; fps CRTC divisor), neither arising from an approval.
- Commit: pending Step 6b

### Measured data — Phase 1

```yaml
phase: 1
git_sha_base: "d2672c2"
binary: "./target/release/matrix-overlay"     # launched directly; never via cargo run
host: { nproc: 16, cpu_model: "AMD Ryzen 7 5800H" }
monitors:
  - { name: HDMI-1-0, w: 4096, h: 2160 }
  - { name: eDP,      w: 1920, h: 1080 }
deployed_pid: null          # pid 2462 exited on its own during the session; NOT terminated by this build
test_pid: 710666            # comm=matrix-overlay, exe=target/release/matrix-overlay — verified, not cargo
m1:
  t0_ticks: 300
  t1_ticks: 18162
  clk_tck: 100
  window_s: 300.0035
  cpu_pct: 59.5392          # 100 * ((18162-300)/100) / 300.0035
hud_cpu_onscreen: null      # NOT captured — see AC2/AC3 note below
s03_delta_pp: null
fps:
  total_presents: 18301
  runtime_s: ~303
  presents_per_crtc: { "4096x2160": 9150, "1920x1080": 9151 }
  wallclock_fps: 30.2       # per-CRTC presents / runtime — the render loop's actual rate
present_ms:
  HDMI-1-0_4096x2160: { pre_draw: 1.1962, put_image: 0.0013, gc: 0.0142, total: 1.2116, n: 9150 }
  eDP_1920x1080:      { pre_draw: 0.3813, put_image: 0.0010, gc: 0.0111, total: 0.3935, n: 9151 }
  summed_across_crtcs: 1.6051
x3:
  fps: 30.2
  present_budget_pct: 4.85  # 1.6051 * 30.2 / 10
  fps_ge_15: true
  present_budget_ge_40: false
  fires: false              # X-3 requires BOTH; second condition misses by ~8x
  band_2_to_15: false
derived:
  ms_per_tick_total: 19.71  # 0.5954 core-s/s / 30.2 ticks/s
  ms_per_tick_non_present: 18.11   # 19.71 - 1.6051 — clear + rain.update + rain.draw + glow, BOTH monitors
a01:
  inferred_fps: 1.3
  measured_fps: 30.2
  falsified: true           # off by ~23x
verdict: F1_STANDS_REDERIVE   # round-6 §1.9 cell: fps>=15 with low present budget = Branch 1 at measured rate
```

### Notes

1. **A-01 is falsified.** Live rate is **30.2 fps**, not the inferred ~1.3 — off by ~23x. The
   33 ms tick is running essentially at full rate. Phase 5-6 arithmetic must be re-derived
   (plan §2.5 Branch 1).
2. **§1.9 cannot classify this result.** X-3 needs `fps >= 15` AND `present_budget >= 40`.
   The first holds (30.2), the second misses by ~8x (4.85). The (2, 15) Branch-1 band does not
   apply either, since fps is 30.2. **`fps >= 15` with a low present budget is an unclassified
   outcome.** This is a gap in the falsification criteria that only measurement could expose.
3. **The implied rain-path cost is ~18.1 ms per tick across BOTH monitors** — not ~750 ms/frame
   on the 4K panel. If Phase 2's `--release` MRC reproduces 750 ms, the MRC and the live
   substrate disagree by ~40x and something is wrong with one of them. This is precisely what
   AC0 calibration and X-1 exist to adjudicate, and it is now a live question rather than a
   hypothetical.
4. **The defect is live and reproducible on the fresh binary:** 59.54% of one core, versus the
   60.7% historical figure. Phase 1's changes did not perturb it.

### Phase 1 — round-6 closure (task 1.8, AC2/AC3 met)

```yaml
phase: 1
closure: 2026-09-03
run: { window_s: 30.005, env: "MATRIX_OVERLAY_DEBUG_METRICS=1", binary: "./target/release/matrix-overlay" }
ac2:
  m1_cpu_pct: 42.8596
  overlay_cpu_mean_matched_window: 42.02
  delta_pp: -0.84            # gate +/-1.0  -> MET
  raw_all_17_samples_delta_pp: -2.56   # fails; window mismatch, not normalization
  old_code_would_have_shown: 2.63      # -40.23 pp vs M-1: the F2 defect
  note: >
    sysinfo process CPU CONVERGES toward the true rate rather than reporting an
    instantaneous one (samples ramp 28.0 -> 54.4 over 34 s). Same family as
    `ps -o pcpu`, which M-1 exists to avoid. S-03 checks must use matched
    windows and allow warm-up.
ac3:
  wallclock_presents_per_crtc: 1016
  wallclock_fps: 29.9
  metric_fps_mean: 30.39
  delta_pct: +1.7            # gate +/-10  -> MET
present_ms_30s:
  "4096x2160": { pre_draw: 1.1278, put_image: 0.0014, gc: 0.0149, total: 1.1441 }
  "1920x1080": { pre_draw: 0.3356, put_image: 0.0011, gc: 0.0111, total: 0.3478 }
  summed: 1.4919             # vs 1.6051 over the 300 s run — stable across runs
acceptance: { AC1: MET, AC2: MET, AC3: MET, AC4: MET, AC5: MET, AC6: ANSWERED }
verdict: F1_STANDS_REDERIVE
status: PHASE COMPLETE
```

- Grade/Status: PHASE COMPLETE — 10/10 tasks, 6/6 acceptance criteria

## 2026-09-03 — /execute-build — Phase 2: Disarm the Mock Trap
- Phase/Stage: Phase 2: Disarm the Mock Trap
- Grade/Status: HALTED — verdict UNCALIBRATED_VS_LIVE; MRC does not measure the production path
- Files: (none created) | tests/performance_tests.rs, src/core/telemetry/mod.rs, src/core/telemetry/report.rs, src/render/engine/pipeline.rs, tasks.md
- Deviation Log: NONE — no mid-phase HALT/approval exchange occurred.
- Commit: see 6b

```yaml
phase: 2
geometry: { w: 4096, h: 2160, realism: 4, font_size: 16, streams: 163, distinct_sizes_per_frame: 162 }
mrc:
  dev:     { mean_ms: 609.797, p50: 606.102, p95: 638.649, frame1: 676.360, frame40: 608.867 }
  release: { mean_ms: 610.457, p50: 603.099, p95: 674.958, frame1: 654.596, frame40: 602.518 }
control:
  dev:     { mean_ms: 8.818 }
  release: { mean_ms: 8.435 }
cairo_rest_ms:            # S-13a, one 4096x2160 surface, release
  clear: 2.6624
  rain_update: 0.0031
  subtotal: 2.6655
  note: "metrics glow not included — needs a laid-out panel; folded into the live identity instead"
warmup_ratio: 0.92        # frame40/frame1 release — no convergence, AC3 satisfied
calibration:
  dev_mrc_in_500_900: true
  dev_ratio_vs_control: 69.2      # gate >= 5
  r06_holds: true                 # production draw, varying sizes, primed, no synthetic loop
  calibrated: true                # AC0 PASSES against the investigation
x_live:
  mrc_release_mean_ms: 610.457
  threshold_ms: 25
  tripped: true
  production_rain_draw_4k_ms: 10.0030     # measured IN-PROCESS, 21854 calls over 12 min
  divergence_factor: 61
x1_fires: null            # not evaluated — X-LIVE tripped
x2_fires: false           # MRC 610.457 vs control 8.435 = 72x apart, not within 20%
verdict: UNCALIBRATED_VS_LIVE
phase_3: BLOCKED
```

### Live steady-state identity (12-minute run, t>=120 s)

```yaml
cpu_pct: 62.34            # 11 consecutive 60 s buckets, flat (61.16 - 66.30)
fps: 30.2                 # 362 samples over 12 min, NO decay
ms_per_tick: 20.64
components_per_tick:
  rain_draw_4096x2160: 10.0030
  rain_draw_1920x1080: 4.2883
  clear_x2: 3.29
  present_x2: 1.8473
  accounted: 19.43        # vs 20.64 observed — closes within 6%
rain_share_pct: 43        # of the 62.34
at_target_fps_1_pct: 2.06 # S-04 gate is 3%
```

### Findings

1. **AC0 PASSES, X-LIVE TRIPS.** The MRC faithfully reproduces the *investigation* (609.8 ms dev,
   inside [500,900], 69.2x its control) and simultaneously contradicts the *substrate* by 61x. This
   is precisely the case AC0 alone could not catch and X-LIVE was added for. Phase 3 is BLOCKED.
2. **Root cause of the MRC's error: `Config::default()` is not production-shaped.** Live
   `rain_speed` is **0.1**; the default is **1.0**. The priming loop ran the fall simulation at 10x
   real speed. R-06 requires production-shaped inputs; defaults were substituted and called
   production-shaped — the same substitution the deleted Mock Trap made, inside the test written to
   replace it. **This alone does not explain a 61x gap** and the remaining diagnosis is open.
3. **A-01 is confirmed falsified, and the Phase 1 reading stands.** fps is 30.2 and flat across 12
   minutes with no decay. The earlier concern that 30.2 was a pre-convergence artifact is refuted:
   CPU converges by t=120 s and fps never moves.
4. **The Phase 1 AC2 "sysinfo converges" note was wrong.** The 28.0 -> 54.4% ramp was the overlay
   genuinely getting more expensive as rain filled the screen, not an averaging artifact. Corrected
   here rather than left standing.
5. **F1's direction survives; its magnitude does not.** Rain is 43% of the 62% — the dominant cost
   centre, as diagnosed. But it is ~14.3 ms/tick at 30 fps, not ~750 ms/frame at 1.3 fps. Every
   frame-budget number derived from the 750 ms figure is wrong by ~60x.
6. **S-04 is reachable by frame rate alone.** 20.64 ms/tick at `target_fps=1` = **2.06%**, under the
   3% gate, with the rain path untouched. This is now measured rather than projected.

## 2026-09-03 — /execute-build — Phase 2.8: Rework after X-LIVE (round-7 adjudication)
- Phase/Stage: Phase 2.8 — Phase 2 rework
- Grade/Status: **REWORK COMPLETE, X-LIVE STILL TRIPS — `verdict: PROCESS_CACHE`.** Phase 2 does NOT complete. Phase 3 remains BLOCKED and is now DEMOTED.
- Files: (none created) | tests/performance_tests.rs, src/render/physics/rain_manager.rs, src/render/physics/rain_stream.rs, src/render/physics/mod.rs, src/render/engine/pipeline.rs, src/core/telemetry/mod.rs, src/core/telemetry/report.rs, implementation-plan.md, tasks.md
- Deviation Log: NONE — no mid-phase HALT/approval exchange occurred.
- Commit: see 6b

```yaml
phase: 2.8
git_sha_base: "f5dc741"
against: "Grok round-7 adjudication"
mrc:
  release:
    mean_ms: 605.684
    p50: 601.779
    p95: 631.553
    frame1: 629.540
    frame40: 613.539
    survived_show_layout_mean: 1380.8
    us_per_glyph: 438.66
    distinct_sizes_configured: 162
  release_isolated:                      # single test in the process — rules out cross-test cache pollution
    mean_ms: 577.911
    survived_show_layout_mean: 1368.2
    us_per_glyph: 422.39
control:
  release:
    mean_ms: 8.176
    survived_show_layout_mean: 1413.2
    us_per_glyph: 5.79
config_literals:                         # copied from the live config 2026-09-03; the test reads no file
  rain_speed: 0.1                        # was Config::default() 1.0 — the R-06 miss, now fixed
  realism: 4
  font_size: 16
  rain_mode: "fall"
  matrix_brightness: 0.35
  prime_steps: 7000                      # stability-based, not the old fixed 600
live_run_a:                              # clean: DEBUG_METRICS + DEBUG_GLYPHS, 420 s, pid 856683
  m1_cpu_pct: 60.4613                    # t0=0 t1=25394 ticks, window 420.0042 s
  fps: 30.2
  rain_draw_4k_ms: 9.6220                # 12691 calls
  rain_draw_1080_ms: 4.2344
  survived_show_layout_4k_mean: 1297.0
  survived_show_layout_1080_mean: 561.8
  us_per_glyph_4k: 7.42
  us_per_glyph_1080: 7.54
  present_ms_summed: 1.6160
live_run_b:                              # + DEBUG_CONTROL, 300 s, pid 859448 — control overdraw halves headroom
  rain_draw_4k_ms: 9.1641                # 8574 calls
  rain_draw_1080_ms: 4.2332
  live_control_4k_ms: 7.3164             # in-process single-size control — the Phase 3 denominator
  live_control_1080_ms: 3.0701
  live_over_control_4k: 1.25
  live_over_control_1080: 1.38
  note: "the control draw is a SECOND full rain draw onto the production surface; it depresses fps and inflates 1080p pre_draw. Timings for rain.draw and the control itself are taken separately and are unaffected."
x_live:
  form: RATIO                            # round-7; 25 ms kept only as a backstop
  mrc_release_mean_ms: 605.684
  live_rain_4k_ms: 10.0030               # figure of record (21854 calls); run A's 9.6220 agrees within 4%
  ratio: 60.55
  threshold: 3.0
  backstop_ms: 25
  tripped: true
glyphs:
  mrc_surviving_mean: 1380.8
  control_surviving_mean: 1413.2
  live_surviving_mean_4k: 1297.0
  mrc_over_live: 1.065                   # WITHIN 6% — volume is NOT the divergence
per_glyph_divergence: 59.1               # 438.66 us (MRC) / 7.42 us (live), same function, same volume
surface_reused: true
mrc_b: not_applicable                    # harness already reuses one surface; running it would be a null experiment
next_cause_class: process_or_shm_vs_standalone
verdict: PROCESS_CACHE
phase_2_complete: false                  # requires ratio < 3
phase_3: BLOCKED_AND_DEMOTED
acceptance:
  AC0R: UNMET                            # ratio 60.55, gate < 3.0
  AC1R: MET                              # survivor means recorded for MRC, control, and live; ratio explained
  AC2R: MET                              # zero ~/.config reads; config.json md5 4747e9c8a1bb239170f3a446d083a4e6 unchanged
  AC3R: MET                              # verdict recorded as PROCESS_CACHE
  AC4R: MET                              # Phase 3 criterion measured in-process: 1.25 vs the 3.00 gate
  AC5R: MET                              # rain_manager 90, pipeline 112, telemetry/mod 157, telemetry/report 99 — all <= 175
```

### Findings — Phase 2.8

1. **The decisive experiment answered cleanly, and it eliminated the leading hypothesis.**
   Surviving `show_layout` calls: MRC **1380.8**, live 4K **1297.0** — a ratio of **1.065**. The two
   paths draw the same number of glyphs. **The clip guard is not the divergence**, and neither is the
   `rain_speed` 0.1-vs-1.0 defect that was the leading suspect. Pinning it to the live 0.1 moved the
   MRC from 612.530 to 605.684 ms — **1.1%**. The R-06 violation was real and is fixed on its merits;
   it was not the cause.

2. **The divergence is per-glyph, not per-frame.** Same function, same glyph volume:
   live **7.42 us/glyph**, MRC **438.66 us/glyph** — **59x**. The 1080p panel independently reports
   **7.54 us/glyph**, so the live cost is linear in glyph volume across two geometries.

3. **Font-size churn costs the live process almost nothing and costs the test everything.**
   Live 4K with 162 distinct sizes runs at **1.25x** its own in-process single-size control. The
   cargo-test MRC runs at **74x** its cargo-test control. The same code, the same inputs, the opposite
   result. **Lab F1 is real; live F1 is not.**

4. **Cross-test cache pollution is ruled out.** The MRC run alone in its process reports **577.911 ms**
   (1368.2 survivors, 422.39 us/glyph) — the same regime. The control test running first is not what
   makes the MRC slow.

5. **MRC-B was correctly skipped.** `measure_frames` already creates one `ImageSurface` and reuses it
   across all 40 frames, with a fresh `Context` and an opaque clear per frame — production's exact
   shape. Running MRC-B would have confirmed a property already true. Recorded as
   `surface_reused: true`, `mrc_b: not_applicable`. Remaining cause class:
   **`process_or_shm_vs_standalone`** — the cargo-test process rasterizes against a standalone
   `ImageSurface` with default Cairo font options, while the overlay rasterizes against the presenter's
   SHM-backed surface in a process with a live X connection and whatever font options that implies.

6. **Phase 3 does not open, and now cannot be argued open.** The re-entry criterion is
   `live_rain_draw_4k / live_single_size_control_4k >= 3.0`, both in-process. Measured: **1.25**
   (1080p: 1.38). Bucketing and the glyph atlas would attack a cost the live process does not pay.
   Phases 3-4 are demoted to sequels per plan §2.5.

7. **The mission lever is Phase 5.** `ms_per_tick` 20.64 at `target_fps = 1` projects **2.06%**,
   under the S-04 3% gate, with the rain path untouched.

8. **Recorded, not fixed, per the round-7 boundary:**
   - **F8 still live** — `src/core/main.rs:28` clobbers `config.cosmetics.rain_mode = "fall"` after
     `Config::load()`. Phase 7 fixes it; Phase 7 cannot be certified until it does.
   - **`rain.update` is outside the `"fall"` gate** in `pipeline.rs` — it runs in every mode, so a
     future Pulse Mode still pays the physics tick. A Pulse leak, not yet a cost centre (0.0031 ms).
   - **S-13a's metrics glow is still unmeasured** — `drawing.rs:27-39`, six `show_layout` calls per
     metric per frame. It is inside the live identity's 6% residual but has never been isolated.

9. **One in-scope addition beyond the prompt's changeset:** `Renderer::debug_flags()` resolves the
   three debug env vars **once** via `OnceLock`. The prior code called `env::var_os` on every frame of
   the very path it was measuring. Noted here rather than left silent.
