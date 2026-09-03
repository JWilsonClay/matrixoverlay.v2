# Tasks — Render Substrate Remediation Campaign

**Plan:** [implementation-plan.md](implementation-plan.md) · **Option:** F — Full Concept Realization
**Created:** 2026-09-03 · **Branch:** `refactor/matrixoverlay.v2`
**Receipts:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

> **NOTHING BELOW HAS BEEN IMPLEMENTED.** Every phase is `NOT STARTED`. This is a plan.
>
> **Mission gate:** S-04 — the live process under **3% of one core** — is the single definition of
> done. All other criteria are means to it.
>
> **Order is load-bearing.** Phase 1 repairs the instruments before anything is measured; Phase 2
> writes a failing test before Phase 3 makes it pass. Reordering these defeats their purpose.

---

## Phase 1: Instrumentation Truth

**STATUS: NOT STARTED** — LOE-1 · Resolves F2, F6 · Blocks every subsequent phase

### Objective
Repair both broken measuring devices before touching the code being measured. The 60% defect
survived ~24h because `overlay_cpu` reported it as 3.79% and nothing exposed the frame rate.

### Tasks
- [ ] 1.1 — Fix `OverlayCpuCollector` normalization in [process.rs:27-30](src/metrics/collectors/system/process.rs#L27-L30). Remove the `/ cores` division so the value matches `top` semantics (% of one core). Retain the whole-machine figure only if surfaced under a distinct, differently-labeled metric.
- [ ] 1.2 — Add an inline comment at the fix site recording *why* `sysinfo`'s own doc advice (`traits.rs:358` — "divide by the number of CPUs") is not followed: it yields %-of-machine, while the label "Overlay CPU" invites `top` comparison. Prevents a well-meaning future revert.
- [ ] 1.3 — Add a `Fps` variant to `MetricId` in [metrics/mod.rs](src/metrics/mod.rs) with `from_str`/`as_str`/`label` arms, following the existing `OverlayCpu` pattern exactly.
- [ ] 1.4 — Implement frame-rate measurement in the renderer: a rolling counter over a 1-second window, published to `SharedMetrics`. Source it from the existing `Renderer::frames` counter rather than adding a second one.
- [ ] 1.5 — Register `fps` in [dispatch.rs](src/metrics/dispatch.rs) and [factory.rs](src/metrics/factory.rs), and add it to the GUI metric list in [ui/gui/metrics.rs:16](src/ui/gui/metrics.rs#L16).
- [ ] 1.6 — Unit test: `overlay_cpu` normalization returns a %-of-one-core value on a known synthetic input.

### Acceptance criteria (MRC)
- [ ] **AC1** — `cargo test` passes.
- [ ] **AC2** — S-03: with the new binary running, `overlay_cpu` reads within **±1 percentage point** of `ps -o pcpu= -p <pid>`. *Command:* `ps -o pcpu= -p $(pgrep -f matrix-overlay)` compared against the on-screen value.
- [ ] **AC3** — S-06: the `fps` metric is within **±10%** of a wall-clock frame count taken over 10 seconds.
- [ ] **AC4** — C-02: the user's existing `~/.config/matrix-overlay/config.json` still parses unmodified.

### Forward contract to Phase 2
A truthful `overlay_cpu` reading and a directly readable `fps` value exist. **A-01 is now testable:**
if `fps` shows the live rate is not ~1.3 fps, halt and re-derive the Phase 5–6 frame budget per
Branch 1 of the plan (§2.5) before continuing.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 2: Disarm the Mock Trap

**STATUS: NOT STARTED** — LOE-1 · Resolves MT · Must go RED before Phase 3

### Objective
Replace the test that stood guard over F1 while measuring the one code path production never takes.
This phase's deliverable is a **failing** test.

### Tasks
- [ ] 2.1 — Delete `test_render_optimization_bench` from [performance_tests.rs:61-87](tests/performance_tests.rs#L61-L87). It asserts `< 500ms` for a single font size through a single layout and comments itself as proof that "with caching, we can render 50k glyphs in milliseconds" — a claim about a path `RainManager::draw` does not take.
- [ ] 2.2 — Write `test_rain_frame_cost_mrc`: call **production** `RainManager::draw` at 4096×2160, `realism=4`, `font_size=16`, streams primed to steady-state distribution, 40 consecutive frames. Assert mean **< 20 ms/frame**.
- [ ] 2.3 — Write `test_rain_fixed_size_control`: the identical glyph count at a **single** font size. Label it explicitly a *control, not a validation* — it exists to prove the cost is size-churn rather than glyph volume, and it is the exact shape of the trap being removed.
- [ ] 2.4 — Add a module-level comment stating the anti-Mock-Trap rule: performance assertions must call production code with production-shaped inputs; anything else is labeled a control.
- [ ] 2.5 — Audit the remaining tests in `performance_tests.rs` for the same defect. `test_pulse_mode_efficiency` asserts `proc.cpu_usage() < 1.0` against a mode that is **not implemented** — record the finding and defer the fix to Phase 7.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-08 (first half): `cargo test --test performance_tests test_rain_frame_cost_mrc` **FAILS**, reporting ~750 ms/frame. A passing result here means the test is not exercising the production path — fix the test, not the threshold.
- [ ] **AC2** — The control test passes at ~12 ms, confirming the delta is size-churn.
- [ ] **AC3** — The 40-frame series shows **no warm-up convergence** (frame 40 within 15% of frame 1), confirming cache eviction rather than cold start.
- [ ] **AC4** — R-06: the MRC contains no synthetic glyph loop; it calls `RainManager::draw`.

### Forward contract to Phase 3
A red MRC exists that measures the real path. Phase 3 is complete when and only when it turns green.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 3: Font Size Bucketing

**STATUS: NOT STARTED** — LOE-2 · Resolves F1 (core) · Requires Phase 2 red MRC

### Objective
Eliminate font-cache eviction by collapsing ~147 distinct font descriptions per frame into a small
fixed set backed by persistent layouts. **The first phase that ships a real CPU win.**

### Tasks
- [ ] 3.1 — Create `src/render/physics/glyph_cache.rs` (new module — C-01 forbids growing `rain_manager.rs`, currently 63 lines, toward the 175 limit).
- [ ] 3.2 — Implement size bucketing: map continuous `depth ∈ 0.5..1.2` to **N=6** discrete font sizes. Hold one persistent `pango::Layout` per bucket, each with its `FontDescription` set **once at construction** and never mutated in the frame loop.
- [ ] 3.3 — **C-03 / R-01 — critical:** quantize the *font size only*. `s.depth` must remain continuous for the alpha calculation at [rain_manager.rs:46](src/render/physics/rain_manager.rs#L46) (`s.depth * s.depth * ...`) and for speed. `concept.md` §II.1 requires size/brightness/speed Z-depth correlation; bucketing `depth` itself would flatten it.
- [ ] 3.4 — Add an inline comment at the bucketing site stating rule 3.3 and why — this is exactly where a future reader will be tempted to "simplify" by bucketing `depth` directly.
- [ ] 3.5 — Rewrite the `RainManager::draw` loop ([rain_manager.rs:36-55](src/render/physics/rain_manager.rs#L36-L55)) to select a cached layout per stream instead of calling `desc.set_size` + `layout.set_font_description`.
- [ ] 3.6 — Invalidate and rebuild the cache when `font_size` or the theme changes; wire into the existing `update_config` path in [pipeline.rs:19-26](src/render/engine/pipeline.rs#L19-L26).
- [ ] 3.7 — Unit test bucket mapping: monotonic, total over the full depth range, stable across calls.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-01 / S-08 (second half): `test_rain_frame_cost_mrc` **PASSES** at **< 20 ms/frame** — from a ~750 ms baseline, ≥ 40×.
- [ ] **AC2** — S-10: `wc -l src/render/physics/*.rs` — every file ≤ **175** lines.
- [ ] **AC3** — `cargo test` fully green; no clippy regressions.
- [ ] **AC4** — **R-01 user sign-off (blocking):** side-by-side screenshots, 6 buckets vs current, presented to the user. Z-depth must read as preserved. **Do not proceed to Phase 4 without an explicit answer.** On rejection, raise bucket count and re-present; on second rejection take Branch 2 (§2.5) and skip to Phase 4.
- [ ] **AC5** — Defect class *Mock Trap*: confirm AC1 passes because the renderer changed, not because the threshold moved. The Phase 2 test file must be unmodified in this phase — verify with `git diff --stat tests/`.

### Forward contract to Phase 4
Per-frame rain cost is bounded and measured. A bucket abstraction exists that the atlas will key on
— Phase 4 extends it rather than replacing it.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 4: Glyph Atlas

**STATUS: NOT STARTED** — LOE-2 · Completes F1 · Removes Pango from the frame path

### Objective
Pre-render each `(glyph, size-bucket)` pair once to a cached Cairo surface and blit thereafter,
eliminating text shaping from the per-frame path entirely. Enables *more* buckets than Phase 3
allowed, improving Z-depth fidelity while costing less.

### Tasks
- [ ] 4.1 — Extend `glyph_cache.rs` with `HashMap<(char, u8), ImageSurface>` keyed by glyph and bucket index. Bounded input: Katakana `0x30A1..=0x30F6` (86 glyphs) × N buckets (A-04).
- [ ] 4.2 — Populate lazily on first use, not eagerly at startup — startup latency must not regress.
- [ ] 4.3 — Replace `show_layout` in the draw loop with `set_source_surface` + `paint_with_alpha`, preserving the existing per-glyph alpha from `s.depth`.
- [ ] 4.4 — Raise bucket count to **N=16** now that per-bucket cost is a one-time rasterization, directly serving `concept.md` §II.1 Z-depth fidelity.
- [ ] 4.5 — **R-02:** measure atlas memory (86 × 16 surfaces) and assert against a hard cap. Implement LRU eviction if the cap is exceeded.
- [ ] 4.6 — **Explicit Error Handling (Theme 4):** atlas construction returns `Result`; a failed glyph rasterization falls back to direct `show_layout` for that glyph rather than panicking.
- [ ] 4.7 — Unit test atlas key derivation, population, cap enforcement, and eviction.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-02: `test_rain_frame_cost_mrc` at **< 8 ms/frame** (threshold tightened from 20 ms).
- [ ] **AC2** — A-04 / R-02: measured atlas memory recorded in the receipt and under the declared cap.
- [ ] **AC3** — Startup time to first frame not regressed by more than 100 ms (lazy population working).
- [ ] **AC4** — S-10: all touched files ≤ 175 lines.
- [ ] **AC5** — Visual parity with Phase 3 at 16 buckets, or better.
- [ ] **AC6** — Fallback path exercised by a test that forces a rasterization failure.

### Forward contract to Phase 5
Per-frame render cost is bounded and small. The remaining CPU variable is *how often* frames are
drawn — Phase 5's domain.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 5: Frame Governor

**STATUS: NOT STARTED** — LOE-3 · Resolves F4 · Aligns with concept.md §IV

### Objective
Fix the frame cap that fails open under load, and bring the refresh rate in line with
`concept.md` §IV: *"1Hz or 0.5Hz is sufficient. Avoid 60fps animations."*

### Tasks
- [ ] 5.1 — Fix `spawn_tick_thread` in [threads/mod.rs:114-125](src/core/threads/mod.rs#L114-L125). Time blocked in `send()` on the `bounded(1)` channel is currently counted in `elapsed`, so once a frame exceeds 33 ms the thread sleeps 1 ms and immediately re-queues — the cap disappears exactly when it is needed.
- [ ] 5.2 — Replace the sleep-accumulator with a **monotonic deadline** scheduler: compute the next tick instant from a fixed epoch, sleep until it, and *skip* missed ticks rather than queuing them. **R-03 / C-05:** this must not introduce visible stutter or strobing.
- [ ] 5.3 — Add `general.target_fps: u32` to [config/types.rs](src/core/config/types.rs) with `#[serde(default = "default_target_fps")]` (**C-02 — mandatory**) and a matching entry in [defaults.rs](src/core/config/defaults.rs). Default **10**, honoring §IV while keeping motion legible.
- [ ] 5.4 — **Theme 4:** clamp `target_fps` to `1..=60` on load. A zero value must not divide by zero; an absurd value must not re-create the runaway.
- [ ] 5.5 — Expose `target_fps` in the GUI General tab, following the existing widget/`update_config_from_widgets` pattern in [ui/gui/logic.rs](src/ui/gui/logic.rs).
- [ ] 5.6 — Unit test governor pacing: inject a simulated 200 ms frame; assert the next tick is not issued before the configured interval.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-07: with an injected 200 ms frame, the tick thread never re-queues faster than the configured interval. This is the direct regression test for F4.
- [ ] **AC2** — `target_fps` is honored: measured `fps` (Phase 1) tracks the configured value within ±10%.
- [ ] **AC3** — Clamping verified at boundaries: `0` → 1, `9999` → 60.
- [ ] **AC4** — C-02: existing config without `target_fps` loads and defaults correctly.
- [ ] **AC5** — **R-03 / C-05 user sign-off (blocking):** rain motion at the new rate must read as smooth and non-strobing. ASD guidance is a hard constraint, not a preference.

### Forward contract to Phase 6
Frame cost (Phase 4) × frame rate (Phase 5) is now bounded and tunable. **Evaluate the §2.5 sequel
here:** if S-04 already passes with margin, consult the user before spending a day on Phase 6.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 6: Damage Rendering and Mutex Removal

**STATUS: NOT STARTED** — LOE-3 · Optional per §2.5 sequel · Highest-risk phase

### Objective
Stop doing per-frame work that need not happen per frame. Metrics change on a 2000 ms `update_ms`
interval but are re-rendered on every frame at whatever `target_fps` dictates.

> **Abandonable.** If Phases 3–5 clear S-04 with margin, this phase is optional (§2.5 sequel).
> The CPU target is the mission; this architecture is one method of reaching it.

### Tasks
- [ ] 6.1 — **R-05:** remove the `SharedMetrics` mutex from the render path. [handlers.rs:51-57](src/core/threads/handlers.rs#L51-L57) currently holds the guard across the entire multi-monitor render. Replace with snapshot-and-release: clone the metric map, drop the guard, then render.
- [ ] 6.2 — Add a dirty flag to the metrics panel: re-render text only when the underlying `MetricValue` set changes, not every frame.
- [ ] 6.3 — Cache the rendered metrics panel to a separate Cairo surface; composite it over the rain each frame instead of re-running the glow passes.
- [ ] 6.4 — **R-04:** implement `force_full_redraw` and invoke it unconditionally on config change, `Expose`, resize, theme change, and monitor reconfiguration. Full repaint remains the fallback — damage tracking is an optimization layered over a correct path, never a replacement for one.
- [ ] 6.5 — Verify the glow-pass reduction: [components.rs](src/render/layout/components.rs) draws 5 glow passes per metric per frame (from the live config's `glow_passes`). With caching these run only on change.
- [ ] 6.6 — Test: assert no stale-pixel artifacts across a config change, an expose event, and a resize.

### Acceptance criteria (MRC)
- [ ] **AC1** — Metrics panel re-renders only on value change; verified by an instrumented counter over 100 frames at `update_ms: 2000` — expect ≈ 1 panel render per 2 s, not 1 per frame.
- [ ] **AC2** — **R-04:** no stale pixels after config change, expose, or resize. This is the failure mode that makes damage rendering dangerous; it must be tested, not assumed.
- [ ] **AC3** — **R-05:** no mutex held across any Cairo call. Verify by inspection and by a test that renders while the metrics thread writes concurrently.
- [ ] **AC4** — Measured CPU improves or holds versus Phase 5. A regression here triggers revert to the Phase 5 commit (§1.7).
- [ ] **AC5** — S-10: all touched files ≤ 175 lines.

### Forward contract to Phase 7
The render loop does minimal per-frame work, and a static-content rendering path exists — which is
precisely what Pulse Mode requires.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 7: Pulse Mode

**STATUS: NOT STARTED** — LOE-4 · Resolves Ghost Logic · concept.md §II.1

### Objective
Implement the mode `concept.md` §II.1 promises — *"static, pulsing glyphs for <0.5% CPU impact"* —
and which `test_pulse_mode_efficiency` has been asserting against for months. Today
[pipeline.rs:35](src/render/engine/pipeline.rs#L35) draws rain only when `rain_mode == "fall"`;
every other value silently draws **nothing**. The mode is not implemented, it is merely absent.

### Tasks
- [ ] 7.1 — Implement `rain_mode: "pulse"` as a real branch: static glyph positions with a slow global alpha oscillation, no per-frame physics and no glyph churn.
- [ ] 7.2 — **C-05 (ASD):** the pulse must be a slow, smooth alpha ramp. `concept.md` §IV requires *"No flashing or blinking elements."* This constraint governs the implementation, not just the review.
- [ ] 7.3 — Reuse the Phase 4 glyph atlas — pulse mode blits the same cached surfaces at a varying alpha.
- [ ] 7.4 — Make the unhandled-`rain_mode` case explicit: an unknown value must log a warning and fall back to a known mode rather than silently rendering nothing. This is the Ghost Logic guard for the whole setting.
- [ ] 7.5 — Fix `test_pulse_mode_efficiency` (deferred from task 2.5) to exercise the now-real mode instead of an empty branch.
- [ ] 7.6 — Expose mode selection in the GUI Cosmetics tab.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-05: live process with `rain_mode: "pulse"` measures **< 0.5% of one core** over a 5-minute steady state. *Command:* `ps -o pcpu= -p $(pgrep -f matrix-overlay)`.
- [ ] **AC2** — **R-09:** AC1 is a measured CPU reading with the mode visibly active on screen — not a code-exists check. The whole point of this phase is that the previous "implementation" was absent while a test passed.
- [ ] **AC3** — C-05: pulse period ≥ 2 s with a smooth ramp; no discontinuous alpha steps.
- [ ] **AC4** — An unknown `rain_mode` logs a warning and falls back, rather than rendering an empty screen.

### Forward contract to Phase 8
Multiple render modes with distinct, measured cost profiles exist — the raw material the
Performance Presets need in order to mean something.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 8: Wire the Performance Presets

**STATUS: NOT STARTED** — LOE-4 · Resolves F5, F7 · Ghost Logic closure

### Objective
Close the dead loop: three GUI buttons connected to nothing, writing to a config field read nowhere.
[advanced.rs:12-19](src/ui/gui/advanced.rs#L12-L19) builds Minimal/Medium/Extreme under a
"Performance Presets" label and returns them as `adv_w`; [mod.rs:79](src/ui/gui/mod.rs#L79) connects
only `adv_w.4` (Purge Logs). `adv_w.0/.1/.2` have **no handler at all** — clicking them does nothing.
Meanwhile `cosmetics.perf_preset` (declared [types.rs:98](src/core/config/types.rs#L98), defaulted
[types.rs:113](src/core/config/types.rs#L113)) is read nowhere in `src/`. The user's live config
says `"perf_preset": "medium"`, set by nothing and read by nothing.

### Tasks
- [ ] 8.1 — Connect `adv_w.0`, `adv_w.1`, `adv_w.2` to `GuiEvent` variants, following the `adv_w.4` → `GuiEvent::PurgeLogs` pattern already in [mod.rs:79](src/ui/gui/mod.rs#L79).
- [ ] 8.2 — Define preset semantics over the knobs built in Phases 3–7:

  | Preset | `target_fps` | `realism` | `glow_passes` | `rain_mode` |
  |---|---|---|---|---|
  | Minimal | 1 | 2 | 1 | `pulse` |
  | Medium | 10 | 4 | 3 | `fall` |
  | Extreme | 30 | 10 | 5 | `fall` |

- [ ] 8.3 — Make `perf_preset` authoritative on load: applying a preset writes the derived values into config and persists atomically via the existing `.tmp`-then-rename path in [config/storage.rs](src/core/config/storage.rs).
- [ ] 8.4 — Handle the divergence case explicitly: when individual settings are edited after a preset is applied, set `perf_preset` to `"custom"` rather than leaving a stale label claiming otherwise.
- [ ] 8.5 — Reflect the active preset in the GUI on open, so the displayed state matches the config.
- [ ] 8.6 — **§2.5 branch:** if the user decides the presets are unwanted, **remove** the three buttons and the `perf_preset` field entirely. Ghost Logic is resolved by implementation or by deletion — never by leaving it in place.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-09: clicking each preset changes `~/.config/matrix-overlay/config.json` on disk **and** produces a visible render change. Both halves required — a config write with no render effect is the same Ghost Logic in a new location.
- [ ] **AC2** — Measured CPU differs materially between Minimal and Extreme, confirming the presets drive real cost.
- [ ] **AC3** — Editing an individual setting after applying a preset sets `perf_preset: "custom"`.
- [ ] **AC4** — C-02: config round-trips through save/load without loss.
- [ ] **AC5** — `grep -rn "perf_preset" src/` shows the field is **read**, not merely declared — the direct inverse of the F5 finding.

### Forward contract to Phase 9
A feature-complete binary satisfying S-01, S-02, S-03, S-05, S-06, S-07, S-09.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 9: Deploy and Live Verification

**STATUS: NOT STARTED** — LOE-5 · Resolves F3 · Requires user approval (C-06)

### Objective
Replace the stale deployed binary and verify the mission gate against the **live** process. Every
prior phase measured a test harness; this one measures reality.

> **C-06 — gated on explicit user approval.** This restarts the running desktop overlay.

### Tasks
- [ ] 9.1 — **Rollback preparation first (§1.7):** `cp ~/.local/bin/matrix-overlay ~/.local/bin/matrix-overlay.pre-remediation` and `cp ~/.config/matrix-overlay/config.json ~/.config/matrix-overlay/config.json.pre-remediation`.
- [ ] 9.2 — Confirm F3 is resolved: the rebuild necessarily includes commit `380107f` (SHM `pre_draw` synchronization). The binary deployed on 2026-05-21 at 15:17:34 predates that 15:25:58 commit, so the known SHM race documented in [pitfalls.md](docs/pitfalls.md) is live in the currently running process.
- [ ] 9.3 — Run `scripts/install.sh` (`cargo build --release` → `cp` to `~/.local/bin/` → autostart).
- [ ] 9.4 — Request user approval, then restart the overlay.
- [ ] 9.5 — Measure the live process over a 5-minute steady state at the default preset.
- [ ] 9.6 — Measure again with `rain_mode: "pulse"`.
- [ ] 9.7 — Cross-check `overlay_cpu` and `fps` on screen against `ps` — closing the loop on the two instruments repaired in Phase 1.
- [ ] 9.8 — Record all readings in the receipt, including any criterion that **failed**.

### Acceptance criteria (MRC)
- [ ] **AC1** — **S-04 — THE MISSION GATE:** live process **< 3% of one core**, sustained over 5 minutes. *Command:* `ps -o pcpu= -p $(pgrep -f matrix-overlay)`. Baseline for comparison: **60.7%**.
- [ ] **AC2** — S-05: `< 0.5%` in Pulse Mode.
- [ ] **AC3** — S-12: `stat -c %y ~/.local/bin/matrix-overlay` is newer than `git log -1 --format=%ci`.
- [ ] **AC4** — S-03 confirmed live: on-screen `overlay_cpu` within ±1pp of `ps`.
- [ ] **AC5** — No visual regression versus the pre-remediation overlay; user confirms.
- [ ] **AC6** — Defect class *Hallucinated Success* / *Sound Effect Execution*: measurements are taken from the **live deployed process**, never from a test harness or a dev build.
- [ ] **AC7** — **§2.5 halt condition:** if AC1 fails while the MRC is green, **halt the campaign and do not document success.** A gap between MRC-green and live-red proves a second cost centre exists outside the rain path, and re-opens investigation.

### Forward contract to Phase 10
Verified live measurements exist to document — including any failures.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase 10: Documentation and Knowledge Capture

**STATUS: NOT STARTED** — LOE-5 · Prevents recurrence

### Objective
Record the mechanism so the next agent does not re-derive it, and correct the two documents that
contributed to this defect surviving.

### Tasks
- [ ] 10.1 — Add a **"Pango Font Cache Eviction"** entry to [docs/pitfalls.md](docs/pitfalls.md), in the same CONFIRMED-BUG style as the existing SHM and OverrideRedirect entries. Must include: the mechanism (cycling N distinct `FontDescription`s through one layout evicts each before reuse); the measured cost (~4.8 ms per size change, 0.02 ms for the setter alone — the cost lands on the first `show_layout` after the change, **not** on `set_font_description`); the fix (bucketing + persistent layouts + atlas); and the constraint that `depth` must stay continuous for alpha and speed.
- [ ] 10.2 — Correct [CLAUDE.md](CLAUDE.md)'s Rendering Pipeline section: it states step 1 is "Clear Cairo `ImageSurface` to **transparent**," but [pitfalls.md](docs/pitfalls.md) documents `Operator::Source` + `rgba(0,0,0,1.0)` (**opaque black**) as the CONFIRMED 2026-05-21 fix for a prior regression. The code is correct; the documentation is wrong, and this discrepancy cost investigation time.
- [ ] 10.3 — Document the new config fields (`target_fps`, preset semantics) in CLAUDE.md's Configuration section.
- [ ] 10.4 — Add a **Mock Trap** entry to pitfalls.md describing `test_render_optimization_bench`: a green performance test that measured a single font size through a single layout — the one case the renderer never takes — while the code it named ran ~90× slower than it asserted. Record the rule that replaced it.
- [ ] 10.5 — Update [DevJournal.md](DevJournal.md) with the 2026-09-03 session: discovery, investigation method, root cause, and outcome.
- [ ] 10.6 — Update CLAUDE.md's module map for `glyph_cache.rs`.

### Acceptance criteria (MRC)
- [ ] **AC1** — pitfalls.md contains both new entries with measured numbers, matching the existing entry format.
- [ ] **AC2** — CLAUDE.md's clear-to-transparent statement is corrected and consistent with pitfalls.md.
- [ ] **AC3** — Every new config field is documented.
- [ ] **AC4** — A fresh reader can reconstruct why 147 distinct font sizes per frame cost 750 ms, from the documentation alone.
- [ ] **AC5** — The campaign's final measured numbers are recorded in DevJournal.md, including any unmet criterion.

**Receipt:** `/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

---

## Phase Summary

| Phase | LOE | Resolves | Gate | Risk |
|---|---|---|---|---|
| 1 — Instrumentation Truth | 1 | F2, F6 | S-03, S-06 | Low |
| 2 — Disarm the Mock Trap | 1 | MT | S-08 (red) | Low |
| 3 — Font Size Bucketing | 2 | F1 core | S-01 (< 20 ms) | **R-01** |
| 4 — Glyph Atlas | 2 | F1 complete | S-02 (< 8 ms) | R-02 |
| 5 — Frame Governor | 3 | F4 | S-07 | **R-03** |
| 6 — Damage + Mutex | 3 | architecture | AC1–AC5 | **R-04, R-05** |
| 7 — Pulse Mode | 4 | Ghost Logic | S-05 (< 0.5%) | R-09 |
| 8 — Wire Presets | 4 | F5, F7 | S-09 | Low |
| 9 — Deploy + Verify | 5 | F3 | **S-04 — MISSION** | R-08 |
| 10 — Documentation | 5 | recurrence | AC1–AC5 | Low |

**Blocking user gates:** Phase 3 AC4 (Z-depth), Phase 5 AC5 (motion smoothness), Phase 9 (restart approval).

**Abandonable:** Phase 6, per §2.5 sequel, if S-04 already passes with margin after Phase 5.

**Halt condition:** Phase 9 AC7 — live measurement failing while the MRC is green means a second
cost centre exists. Halt; do not document success.
