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
verdict: UNCLASSIFIED_SEE_NOTE   # X-3 misses; A-01 falsified; §1.9 has no branch for this
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
