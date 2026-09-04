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

## 2026-09-03 — /execute-build — Phase 2 CLOSE + 2.9 probe (round-8)
- Phase/Stage: Phase 2 close (`CLOSED_LAB_DIVERGENT`) + optional 2.9 sidecar
- Grade/Status: PHASE 2 CLOSED — mission deliverables complete; X-LIVE recorded as a finding
- Files: (none created) | implementation-plan.md, tasks.md, docs/pitfalls.md, Cargo.toml, tests/performance_tests.rs, src/render/mod.rs, src/render/engine/pipeline.rs, src/core/telemetry/mod.rs, src/core/telemetry/report.rs
- Deviation Log: NONE
- Commit: see 6b

```yaml
phase: 2
status: CLOSED_LAB_DIVERGENT
x_live: { ratio: 60.55, finding_not_halt: true }
lab_f1:  { mrc_ms: 605.684, control_ms: 8.176,  ratio: 74 }
live_f1: { rain_4k_ms: 9.6220, control_4k_ms: 7.3164, ratio: 1.25 }
phase_3: BLOCKED_AND_DEMOTED
phase_4: BLOCKED_AND_DEMOTED
optional_2_9:
  gtk_init_mrc_ms: 471.426                # also 474.681 / 507.235 across three runs
  baseline_isolated_no_gtk_ms: 577.911
  gtk_init_effect: "-18% — real, but not the mechanism"
  font_options_live: "antialias=Default hint_style=Default hint_metrics=Default subpixel_order=Default"
  font_options_mrc:  "antialias=Default hint_style=Default hint_metrics=Default subpixel_order=Default"
  font_options_identical: true            # eliminates the Cairo-font-options mechanism
  conclusion: >
    Neither E1 nor E2 explains the divergence. gtk::init() recovers ~18%; font
    options are byte-identical on both sides. Leading remaining mechanism is the
    GTK / PangoCairoFontMap / Xft font-map state of the overlay process versus a
    bare test font map — NOT SHM-vs-ImageSurface, which this pass also failed to
    implicate. Chased no further per round-8 Q1.
  pitfalls_stub_written: true             # docs/pitfalls.md, "a cargo test benchmark is not a measurement"
phase_5_blocked_by_2_9: false
```

### Findings — Phase 2 close

1. **E2 eliminated the mechanism I had named.** The round-7 receipt put `process_or_shm_vs_standalone`
   forward with Cairo font options as the concrete suspect. They are **identical** on both sides. The
   surface class is not implicated either. Recorded as a dead end rather than left implying progress.
2. **E1 moved the number without explaining it.** `gtk::init()` in the test drops the isolated MRC
   from 577.911 to ~471-507 ms (~18%). Real, reproducible, and far short of the 59x that would be
   needed. It supports the font-map hypothesis without confirming it.
3. **S-01 and S-08 are relabeled, not deleted.** S-01 becomes `LAB_F1` documentation and gates
   nothing — the live 4K figure (9.6220 ms) is already inside its 20 ms threshold. S-08's
   red-before/green-after transition is **vacated**, because it presumed a Phase 3 that the
   in-process 1.25x ratio has demoted. The half of S-08 that was actually delivered — the deleted
   `test_render_optimization_bench`, the R-06 rule, the labeled control — stands.
4. **New dev-dependency:** `gtk = "0.16"`, test-only, same version as the main dependency so it is
   the same crate instance. Added solely for the E1 probe.

## 2026-09-03 — /execute-build — Phase 5: Frame Governor
- Phase/Stage: Phase 5 — Frame Governor (LOE-3, the mission lever)
- Grade/Status: **TASKS COMPLETE — AC6 UNMET by 0.017 pp. `verdict: S04_UNMET_BRING_RECEIPT`.** AC5 pending user.
- Files: tests/governor_tests.rs | src/core/threads/mod.rs, src/core/config/types.rs, src/core/config/defaults.rs, src/core/config/mod.rs, src/ui/gui/general.rs, src/ui/gui/logic.rs, Cargo.toml
- Deviation Log: NONE
- Commit: see 6b

```yaml
phase: 5
git_sha_base: "1b6745b"
target_fps_default: 1
ac1_governor_holds: true          # rate assertion; red-before-green verified (see finding 1)
ac2:
  target_1:
    wallclock_fps: 0.995          # 601 presents/CRTC over ~604 s runtime
    metric_fps_mean: 1.009        # 150 samples in the M-1 window
    delta_vs_target_pct: +0.9     # gate +/-10 -> MET
    m1_cpu_pct: 3.0166
    window_s: 300.0072
  target_5:
    wallclock_fps: 4.934          # 1500 presents/CRTC over ~304 s runtime
    metric_fps_mean: 4.997        # 70 samples
    delta_vs_target_pct: -1.3     # gate +/-10 -> MET
    m1_cpu_pct: 11.9795           # tracking only; not an S-04 gate
    window_s: 150.0057
ac3_clamp: true                   # 0 -> 1, 9999 -> 60, unit-tested on General::fps and tick_period
ac4_old_config_loads: true        # pre-target_fps JSON parses under deny_unknown_fields, defaults to 1
ac5_user_signoff: pending         # BLOCKING_HITL — not forged, not skipped
ac6:
  projected_pct_at_1: 2.06        # from ms_per_tick 20.64 measured at 30 fps
  measured_m1_at_1: 3.0166
  s04: UNMET                      # gate 3.0 — missed by 0.017 pp (0.55% relative)
decomposition_at_1_fps:
  rain_draw_4k_ms: 10.5461        # 601 calls
  rain_draw_1080_ms: 5.6787
  present_summed_ms: 3.2100
  measured_subtotal_ms: 19.4348   # -> 1.943% at 1 fps
  non_render_floor_pct: 1.073     # 3.0166 - 1.943
  unmeasured_in_subtotal: "clear x2 (~3.29 ms, S-13a) and the metrics glow (never measured)"
verdict: S04_UNMET_BRING_RECEIPT
phase_3: BLOCKED_AND_DEMOTED      # not reopened, not used to explain the miss
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"   # user's config untouched
tests: { lib: 6 pass, asd_tests: 5 pass, governor_tests: 6 pass, metrics_tests: excluded (MT-3, never compiled) }
line_caps: { threads/mod: 160, config/types: 161, config/mod: 120, gui/general: 81, gui/logic: 40, config/defaults: 40 }
```

### Findings — Phase 5

1. **The first S-07 test was a Mock Trap and was caught by the red-check, not by review.**
   It asserted one step — that the next deadline lands after the slow frame and within one period of
   it. Reinstating a deliberately fail-open `next_deadline` (`now + 1ms`) **passed all six tests**,
   because a 1 ms tick also satisfies both conditions. F4's signature is not one early tick; it is an
   **unbounded issue rate under load**. The test was rewritten to drive 20 overrunning frames and
   assert the achieved rate. Re-running the fail-open version now fails with
   `fps=1: 20 frames took 4.8s, under the 19s the rate allows`. Green restored after. This is the
   third time in this campaign a test written to prevent a defect class reproduced it.

2. **The governor holds and tracks.** 0.995 fps at `target_fps=1` and 4.934 at 5 — both inside +/-10%
   on wall-clock presents, and the on-screen metric agrees (1.009, 4.997). The 1 ms fail-open branch
   is gone; missed ticks are skipped rather than queued.

3. **S-04 is missed by 0.017 pp — and the render term is not why.** The 2.06% projection came from
   `ms_per_tick = 20.64` measured at 30 fps. At 1 fps the measured render subtotal is **19.43 ms**
   (1.943%) — the projection's render half was right within 6%. What the budget identity has **no term
   for** is a **non-render floor of ~1.07%**: metrics collectors on the 2 s `update_ms` (including the
   `nvidia-smi` subprocess), the GTK/tray thread, and the XCB event thread. Every one of those is
   frame-rate-independent, so lowering `target_fps` further cannot remove it.

4. **The identity closes almost exactly.** 1.943% render + 1.073% floor = 3.016% against M-1's
   3.0166%. The gap is not measurement noise, and it is not somewhere unknown.

5. **Two known-open items sit inside the residual.** The metrics glow
   ([drawing.rs:27-39](src/render/layout/drawing.rs#L27-L39), six `show_layout` calls per metric per
   frame) has never been measured — S-13a recorded `clear` and `rain.update` only. `clear` itself is
   ~3.29 ms/tick across both surfaces and is absent from the subtotal above. Either could account for
   the miss; neither was chased in this pass.

6. **Phase 3 was not reopened to explain the miss**, per the round-8 instruction. Rain at 1 fps costs
   1.62% of the 3.0166% and the in-process re-entry ratio is still 1.25 against a 3.00 gate.

7. **Recorded, not fixed (unchanged):** F8 (`main.rs` clobbers `rain_mode`), `rain.update` outside the
   `"fall"` gate, the unmeasured glow.

8. **`tests/metrics_tests.rs` still does not compile** (`NvidiaSmiCollector::new_with_command` does not
   exist) — the pre-existing MT-3 finding, excluded from the phase gate as Phase 2 AC3 specifies. Not
   introduced here.

## 2026-09-04 — /execute-build — Phase 5.8: Isolate the residual (round-9)
- Phase/Stage: Phase 5.8 — residual isolation
- Grade/Status: **COMPLETE — four terms named, remainder measured. `decision: PANEL_CACHE`. 5.9 NOT authorized (no evidence collectors dominate M-1).**
- Files: src/core/telemetry/phase58.rs | src/core/telemetry/mod.rs, src/core/telemetry/report.rs, src/render/engine/pipeline.rs, src/metrics/manager.rs, src/metrics/collectors/nvidia.rs, implementation-plan.md, tasks.md, docs/pitfalls.md
- Deviation Log: NONE
- Commit: see 6b

```yaml
phase: 5.8
git_sha_base: "4b6cb16"
target_fps: 1
update_ms: 2000                  # unchanged — 5.9 not authorized, see finding 3
m1:
  clk_tck: 100
  windows_300s:                  # THREE windows, not one — see finding 1
    - { tag: phase5, t0_ticks: 1225, t1_ticks: 2130, window_s: 300.0072, cpu_pct: 3.0166 }
    - { tag: 5.8-a,  t0_ticks: 1243, t1_ticks: 2142, window_s: 300.0064, cpu_pct: 2.9966 }
    - { tag: 5.8-b,  t0_ticks: 1252, t1_ticks: 2151, window_s: 300.0064, cpu_pct: 2.9966 }
  mean_pct: 3.0033
  spread_pp: 0.0200
  over_gate: "1 of 3"
wallclock_fps: 0.995             # 601 presents/CRTC over ~604 s
render:                          # per tick, summed across both CRTCs
  rain_4k_ms: 10.6972
  rain_1080_ms: 5.5156
  present_sum_ms: 3.1148
  pct: 1.9231
clear:                           # F-B — re-measured at 1 fps on the LIVE SHM surfaces
  hdmi_4k_ms: 3.1733
  edp_1080_ms: 0.5355
  sum_ms: 3.7088
  pct: 0.3690
  note: "S-13a's ~3.29 ms was 30 fps on a standalone surface; this supersedes it for the identity"
glow:                            # F-A — never measured before this phase
  hdmi_4k_ms: 0.7420
  edp_1080_ms: 0.9429
  sum_ms: 1.6848
  pct: 0.1676
  note: "the 1080p panel costs MORE than the 4K one — glow is per-metric, not per-pixel"
collectors:                      # F-C
  cycles: 300                    # update_ms=2000 over ~604 s
  tick_mean_ms: 117.9433
  tick_total_wall_s: 35.383
  nvidia_smi_calls: 300
  nvidia_smi_mean_ms: 28.0314
  nvidia_smi_total_wall_s: 8.409
  pct_of_m1: "NOT SEPARABLE — see finding 3. This is WALL time, most of it blocking."
render_subtotal_ms_per_tick: 24.7212
render_subtotal_pct: 2.4598      # rain + present + clear + glow
remainder_pct: 0.5368            # M-1 2.9966 - 2.4598. Collectors + GTK/tray + XCB.
decision: PANEL_CACHE            # clear+glow = 0.5367 >= 0.5 per round-9 §3.4
s05_floor_ok: false              # remainder 0.5368 >= 0.5 -> Phase 7 stays shut, S-05 needs amending
ac5_user_signoff: pending
phase_3: BLOCKED_AND_DEMOTED
gate_moved: false                # 3.0% untouched; 3.0166 not rounded down
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"
line_caps: { telemetry/phase58: 77, telemetry/mod: 158, telemetry/report: 142, pipeline: 131, manager: 92, nvidia: 47 }
```

### The identity, as named quantities

```
rain     16.2128 ms/tick -> 1.6132 %
present   3.1148 ms/tick -> 0.3099 %
clear     3.7088 ms/tick -> 0.3690 %
glow      1.6848 ms/tick -> 0.1676 %
                            -------
render subtotal             2.4598 %
remainder (collectors + GTK + XCB)  0.5368 %
                            -------
                            2.9966 %  = M-1, exactly
```

### Findings — Phase 5.8

1. **S-04 is inside the run-to-run noise band, and that is the real headline.** Three 300 s M-1
   windows at `target_fps=1`: **3.0166, 2.9966, 2.9966** — mean **3.0033**, spread **0.020 pp**. The
   gate is 3.0. **One of three runs is over it.** The third run was taken specifically because two
   runs straddling a gate cannot decide it. S-04 is therefore neither met nor missed at any useful
   confidence: it is *at* the gate. Declaring it met on the two low readings would be rounding
   3.0166 down by selection, which round-9 explicitly forbids. `decision` is set from the term
   analysis, not from the straddle.

2. **The 1.073% "floor" was half render.** Round-9's ruling was correct: `clear` (0.3690%) and glow
   (0.1676%) are rate-dependent and belong in `cairo_rest_ms`. Together they are **0.5367%** — almost
   exactly the 0.536 pp by which the old subtraction overstated the floor. **The true floor is
   0.5368%, not 1.073%.**

3. **F-C cannot be converted into an M-1 percentage, and reporting one would be wrong.** The collector
   cycle costs **117.94 ms of WALL time**, of which **28.03 ms is `nvidia-smi`**. That subprocess's
   CPU is charged to a **child process** and never appears in this process's `utime + stime`, which is
   what M-1 reads (fields 14+15, not `cutime`/`cstime`). Most of the remaining wall time is the
   collector thread *blocked*, not running. So the honest statement is: **collectors contribute to the
   0.5368% remainder along with GTK/tray and XCB, and this pass cannot say by how much.**
   Consequently **5.9 is NOT authorized** — round 9 requires evidence that collectors dominate, and
   this is not that evidence. Raising `update_ms` would cut dGPU wakeups and system-wide load (a real
   benefit `CLAUDE.md` already argues for) while possibly barely moving M-1.

4. **The glow is not where it was expected, and it is small.** 1.6848 ms/tick total — **0.1676%**, the
   smallest of the four terms. It had been named as unmeasured in three consecutive receipts and
   treated as the leading suspect for the residual. It is not. Note also that the **1080p panel costs
   more glow than the 4K one** (0.9429 vs 0.7420 ms): glow scales with metric count and text length,
   not pixels, and the eDP screen carries more metrics.

5. **`clear` is the larger of the two, and a panel cache will not touch it.** At 3.7088 ms/tick
   (0.3690%) it is more than twice the glow. It is an opaque full-surface `paint` of 4096×2160 plus
   1920×1080 and cannot be cached away — only damage tracking (the old Phase 6.1) removes it. So
   `decision: PANEL_CACHE` is set by the §3.4 table, but its realistic ceiling is the **0.1676%** of
   glow, not the 0.5367% of `clear + glow`. Recorded rather than left to be discovered later.

6. **`s05_floor_ok: false`.** The remainder is 0.5368% and S-05 demands `< 0.5%` for Pulse Mode. Even
   a Pulse Mode that draws *nothing* cannot meet S-05 against this floor. **Phase 7 must not open**
   until either the floor drops below 0.5% or S-05 is explicitly amended — exactly the condition
   round 9 anticipated.

7. **S-13a's `clear` figure is superseded.** It recorded ~3.29 ms at 30 fps on a standalone
   `ImageSurface`; the live SHM figure at 1 fps is **3.7088 ms**. Close, but measured on the right
   surface at the right rate, and the identity should use this one.

8. **Recorded, not fixed (unchanged):** F8 (`main.rs` clobbers `rain_mode`), `rain.update` outside the
   `"fall"` gate.

## 2026-09-04 — /execute-build — Round 10: S-04 exception, F8, Phase 8
- Phase/Stage: Phase 8 — Performance Presets (Medium + Extreme); plus the S-04 written exception and the F8 fix
- Grade/Status: **PHASE 8 TASKS COMPLETE (Minimal deferred). S-04 = `S04_AT_GATE`. S-05 amended. F8 FIXED.**
- Files: src/core/config/presets.rs | implementation-plan.md, tasks.md, docs/pitfalls.md, src/core/main.rs, src/core/init.rs, src/core/mod.rs, src/core/config/mod.rs, src/core/threads/handlers.rs, src/render/engine/pipeline.rs, src/ui/gui/{mod,advanced,logic}.rs, src/metrics/mod.rs · DELETED: src/core/timer.rs, src/metrics/factory.rs
- Deviation Log: NONE
- Commits: 88b6f38 (docs), 13c1463 (F8), see 6b (Phase 8)

```yaml
phase: 8
git_sha_base: "95e7bac"
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
s04_spread_pp: 0.020
gate_numeral: 3.0                 # unchanged; 3.0166 not rounded down
s05: AMENDED                      # 0.5% whole-process bound retired as unsatisfiable
f8:
  line_removed: true
  git_log_S_reason: >
    d2f61a1 (2026-02-28, "Finished Build PRE Release") introduced it under the
    comment "FORCE OVERRIDE: Ensure rain is enabled for verification". A
    debugging override, never removed. Its companion from the same commit and
    comment (realism_scale = 8) was cleaned up; this one survived ~7 months.
  in_process_survives_restart: true
  evidence: >
    Throwaway $HOME with rain_mode="pulse": logs "Effective rain_mode after
    config load: pulse", and the telemetry summary contains NO X-LIVE section —
    rain.draw was never called. Under the old line it would have drawn fall rain.
presets:
  medium:
    writes_config: true           # target_fps=1 realism=4 glow=3 rain_mode=fall
    live_wallclock_presents_per_crtc: 46      # over 45 s => 1.02 fps
    live_metric_fps: 1.0
    fps_unchanged_1: true
  extreme:
    writes_config: true           # target_fps=30 realism=10 glow=5 rain_mode=fall
    live_metric_fps: "29.5 - 30.2 at steady state"
    whole_run_wallclock_fps: 12.6 # DILUTED by startup before the first present; not the steady rate
    label: "Extreme (exceeds the ambience budget)"
    s04_exempt: true
  minimal: DEFERRED               # needs Pulse Mode; button logs and does nothing. Screen NOT blanked.
gl2:
  perf_preset: wired              # READ in ui/gui/logic.rs:38 and ui/gui/advanced.rs:24
  show_monitor_label: wired       # render/engine/pipeline.rs:120
  build_logging_enabled: wired    # core/init.rs:22
  timer_rs: deleted
  factory_rs: deleted_with_timer
ac5_user_signoff: pending
phase_7: SHUT
phase_9: SHUT
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"
tests: { lib: 11 pass, asd_tests: 5 pass, governor_tests: 6 pass }
line_caps: { presets: 122, handlers: 160, pipeline: 131, gui/mod: 104, gui/advanced: 38, gui/logic: 48, main: 137, init: 110 }
```

### Findings — Round 10

1. **`timer.rs` was a second, dead copy of the metrics loop — including the F4 bug.** It carried the
   same `else { thread::sleep(Duration::from_millis(1)); }` fail-open that Phase 5 removed from the
   tick thread. It had **no callers**; `factory.rs::create_collectors` had exactly one caller, and it
   was `timer.rs`. Both deleted together, as GL-2 required deciding them together. Had Phase 5 not
   found the fail-open in `threads/mod.rs` first, this file would have looked like the fix site.

2. **Neither GL-2 flag could be deleted, and the reason is C-02.** `show_monitor_label` and
   `build_logging_enabled` are both **present in the user's live `config.json`**. Removing either
   field from the struct would make `#[serde(deny_unknown_fields)]` reject that file on the next
   start — the config would fail to load, not degrade. "Wire or delete" therefore had only one safe
   branch here, and both are wired. `show_monitor_label` is `false` in the user's config, so wiring it
   changes nothing they will see.

3. **The preset verification script initially re-derived the preset table in Python** — a second copy
   of the production values, asserted against. That is the campaign's standing-rule defect for the
   fourth time, caught before it ran. Split instead: the table is asserted by unit tests that call
   production `presets::apply` directly, and the live script only *writes a config and watches
   behaviour*, asserting nothing about the table.

4. **Extreme's whole-run wall-clock rate (12.6 fps) is not its rate.** The 45 s run includes startup —
   GTK init, window creation, the weather fetch — before the first present. Steady-state metric `fps`
   reads **29.5–30.2** against a target of 30. Reported as both numbers rather than the flattering one.

5. **Minimal does not blank the screen.** The button logs *"Minimal preset requires Pulse Mode
   (Phase 7); not applied"* and returns; `presets::apply` returns `false` for it and mutates nothing.
   The GUI label says "needs Pulse Mode — not yet available", so the control is honest rather than
   inert-and-silent.

6. **`clear`'s cost is why Phase 6.1 cannot close S-04, and this is now written into §2.5.** Damage
   tracking skips the opaque paint only when the surface is *not* fully dirtied; falling rain dirties
   all of 4096×2160 and 1920×1080 every tick. Phase 6.1 is a Pulse/static-mode lever, recorded as a
   sequel there rather than against S-04.

7. **Recorded, not fixed (unchanged):** `rain.update` still runs outside the `"fall"` gate in
   `pipeline.rs` — a Pulse leak that will matter in Phase 7 and costs 0.0031 ms today.
