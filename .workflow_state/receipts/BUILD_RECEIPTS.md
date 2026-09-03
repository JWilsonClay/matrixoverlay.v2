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
