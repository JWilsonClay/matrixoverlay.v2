# Tasks — Render Substrate Remediation Campaign

**Plan:** [implementation-plan.md](implementation-plan.md) · **Option:** F — Full Concept Realization
**Created:** 2026-09-03 · **Branch:** `refactor/matrixoverlay.v2`
**Receipts:** `receipts/BUILD_RECEIPTS.md` — repo-root relative, created on first write by Phase 1.
*(Audit: the previously named `.workflow_state/receipts/` directory does not exist in this repository and is not created by the planning pass.)*

> **NOTHING BELOW HAS BEEN IMPLEMENTED.** Every phase is `NOT STARTED`. This is a plan.
>
> **Mission gate:** S-04 — the live process under **3% of one core** — is the single definition of
> done. All other criteria are means to it.
>
> **Order is load-bearing.** Phase 1 repairs the instruments before anything is measured; Phase 2
> writes a failing test before Phase 3 makes it pass. Reordering these defeats their purpose.
>
> **Revised 2026-09-03 by adversarial audit.** See "Audit Absorption" at the foot of
> [implementation-plan.md](implementation-plan.md) for what was accepted and what was rejected.
> Three cross-cutting rules now bind every phase:
> 1. **Method M-1** (plan §1.3) is the *only* live-CPU measurement. `ps -o pcpu` is a lifetime
>    average and does not measure a 5-minute window; `top` is a different instrument again.
> 2. **All MRC gates run `--release`.** The deployed binary is `opt-level = "z"` + LTO; a dev-profile
>    timing does not describe it.
> 3. **`cargo test` is not a blanket gate.** `tests/window_integration.rs` maps windows onto the live
>    desktop and asserts a geometry that is false on this host (R-11). No phase may require it green.
>
> **Execution gate (round-2 audit, confirmed round-3, 2026-09-03).** Authorized scope is **Phases 1 and
> 2 only**, then a mandatory stop. At that stop, evaluate **plan §1.9 — Falsification Criteria**: three
> named readings (X-1, X-2, X-3) that would mean **F1 is the wrong root cause**, not merely that the
> arithmetic needs re-deriving. Phase 1 AC6 answers X-3; Phase 2 AC5 answers X-1 and X-2. Phase 1 measures S-13b and converts A-01 from assumption to reading; Phase 2 produces
> the red MRC and measures S-13a. **Do not open Phase 3 on the pre-audit arithmetic** — re-derive
> Phase 5's default `target_fps` from the numbers those two phases actually produce. The 10 fps in task
> 5.3 is a placeholder that the budget identity is expected to lower.

---

## Phase 1: Instrumentation Truth

**STATUS: NOT STARTED** — LOE-1 · Resolves F2, F6 · Blocks every subsequent phase

### Objective
Repair both broken measuring devices before touching the code being measured. The 60% defect
survived ~24h because `overlay_cpu` reported it as 3.79% and nothing exposed the frame rate.

### Tasks
- [ ] 1.0 — Create `receipts/BUILD_RECEIPTS.md` at the repository root on first write. Every later phase appends to it. Do not create `.workflow_state/`; it does not exist and nothing requires it.
- [ ] 1.1 — Fix `OverlayCpuCollector` normalization in [process.rs:27-30](src/metrics/collectors/system/process.rs#L27-L30). Remove the `/ cores` division so the value matches **Method M-1** semantics (% of one core). Retain the whole-machine figure only if surfaced under a distinct, differently-labeled metric.
- [ ] 1.2 — Add an inline comment at the fix site recording *why* `sysinfo`'s own doc advice (`traits.rs:358` — "divide by the number of CPUs") is not followed: it yields %-of-machine, while the metric's on-screen label — **"HUD CPU"** ([metrics/mod.rs:76](src/metrics/mod.rs#L76)) — invites comparison against an external per-core reading. Prevents a well-meaning future revert.
- [ ] 1.3 — Add a `Fps` variant to `MetricId` in [metrics/mod.rs](src/metrics/mod.rs) with `from_str`/`as_str`/`label` arms, following the existing `OverlayCpu` pattern exactly.
- [ ] 1.4 — Implement frame-rate measurement as an **`AtomicU64` incremented inside `Presenter::present`** ([presentation/mod.rs:15](src/render/engine/presentation/mod.rs#L15)) — one count per successful present, per monitor. **Audit correction:** do *not* source it from `Renderer::frames`. That counter is incremented at [pipeline.rs:29](src/render/engine/pipeline.rs#L29) on every `draw()`, which includes `Expose`-driven draws ([handlers.rs:31-37](src/core/threads/handlers.rs#L31-L37)), so it is a draw count, not a present count — and it never reaches `SharedMetrics` at all.
- [ ] 1.4b — **Audit correction — S-06 is otherwise unimplementable.** A value written directly into `SharedMetrics.data` is erased on the next collection tick: [manager.rs:54](src/metrics/manager.rs#L54) does `sh.data = MetricData { values: frame }`, a wholesale replacement built only from the collector list. Publish `fps` through a small `FpsCollector` that reads the atomic, so it lands *inside* `frame` rather than being overwritten by it. This is the one named exception to "collector logic beyond `overlay_cpu` is out of scope" (plan §1.2).
- [ ] 1.5 — Register `fps` in [metrics/mod.rs](src/metrics/mod.rs) (`MetricId` + `from_str`/`as_str`/`label` arms) and [dispatch.rs](src/metrics/dispatch.rs) **only**. **Audit correction:** the previous instruction to edit [ui/gui/metrics.rs:16](src/ui/gui/metrics.rs#L16) and [factory.rs](src/metrics/factory.rs) is *Sound Effect Execution* — [ui/gui/mod.rs:38-53](src/ui/gui/mod.rs#L38-L53) appends only General, Cosmetics, Weather and Advanced, so the Metrics tab is never shown; and `factory::create_collectors` is reached only from [core/timer.rs:19](src/core/timer.rs#L19), which nothing calls. Editing either changes nothing the user or the renderer can observe. Both are recorded as GL-2 and resolved in Phase 8.7.
- [ ] 1.6 — Unit test: `overlay_cpu` normalization returns a %-of-one-core value on a known synthetic input.
- [ ] 1.7 — **S-13b — measure the X-side per-frame cost, per monitor.** On the same temporary `cargo run --release` that AC2/AC3 start, time `Presenter::pre_draw` (the `GetInputFocus` round-trip), `ShmPutImage`, and `CreateGc`/`FreeGc` — **separately for each CRTC**, since a 4096×2160×4 buffer and a 1920×1080×4 buffer are not the same cost. Record `present_ms` per monitor in the receipt. **Implementation constraint (round-3 audit): accumulate internally and print one summary at exit. Do not log per present** — a log line on the path being measured is a new cost centre inside the measurement, and would inflate the very number S-13b exists to establish. **This belongs here, not in Phase 2** *(round-2 audit)*: Phase 2 stands up neither SHM nor RandR, so a present-path number taken inside the MRC harness would be measured off a path production does not take — a new Mock Trap in the act of closing the old one. Phase 1 already has a real connection.

### Acceptance criteria (MRC)
- [ ] **AC1** — `cargo test --release --lib --test performance_tests --test metrics_tests` passes. **Audit (R-11):** not a blanket `cargo test` — `tests/window_integration.rs` maps windows onto the user's live desktop and asserts 1920×1080 at (0,0), which is false on this host (RandR yields 4096×2160 + 1920×1080). Excluded here; quarantined in Phase 3 AC3.
- [ ] **AC2** — S-03 via **Method M-1** (plan §1.3): run a temporary foreground `cargo run --release` — **not** an install, and without terminating the running overlay (C-06 belongs to Phase 9) — then compare the on-screen `HUD CPU` value against two `/proc/<pid>/stat` samples 300 s apart. Within **±1 percentage point**.
  **Pid pinning — round-2 audit; this AC was unsatisfiable as first written.** M-1 requires `pgrep -x matrix-overlay` to return exactly one pid, but this AC deliberately leaves the deployed overlay (pid 2462 today) running *and* starts a second process of the same name. `pgrep -x` would return two, and M-1's guard would exit 1 by construction. **Pin the overlay's own pid — and note that `cargo run --release & TESTPID=$!` does NOT do that** *(round-3 audit)*. `$!` is **cargo's** pid; cargo spawns the overlay as a child, so sampling `/proc/$!/stat` measures cargo sitting idle at ~0% and AC2 would pass falsely — a Hallucinated Success generator in the very AC written to prevent one. Instead: **`cargo build --release` first, then run `./target/release/matrix-overlay & TESTPID=$!` directly**, so `$!` is the overlay. If a wrapper is unavoidable, read the pid from a startup log line the binary emits, or walk cargo's children — never assume `$!`. Sample `/proc/$TESTPID/stat` on that pid. Exclude the deployed instance by its recorded pid, never by a name match over the whole machine. `pgrep -x` remains correct in **Phase 9**, where exactly one instance is running by then.
- [ ] **AC3** — S-06: the `fps` metric is within **±10%** of an independent 10-second wall-clock count of `Presenter::present` calls (log line or test hook). **Not** a `ps` cross-check — `ps` cannot measure frame rate.
- [ ] **AC4** — C-02: the user's existing `~/.config/matrix-overlay/config.json` still parses unmodified.
- [ ] **AC5** — A-01 becomes a measurement rather than an assumption. Record the observed live fps in the receipt. If it is **not** ~1.3 fps, take Branch 1 (plan §2.5) and re-derive the Phase 5–6 frame budget before continuing.
- [ ] **AC6** — **Falsification check X-3 (plan §1.9).** With `fps` (AC3/AC5) and `present_ms` per CRTC (task 1.7) both in hand, evaluate: is `fps ≥ 15` **and** `(present_ms_hdmi + present_ms_edp) × fps ÷ 10 ≥ 40`? If **yes**, present × rate × two CRTCs already accounts for the 61% — **F1 is at most a contributor, not the root cause.** Record the finding and carry it into the Phase 2/3 stop; do not silently continue. This AC has no pass/fail — it has an answer, and the answer routes the campaign.

### Forward contract to Phase 2
A truthful `overlay_cpu` reading and a directly readable `fps` value exist. **A-01 is now testable:**
if `fps` shows the live rate is not ~1.3 fps, halt and re-derive the Phase 5–6 frame budget per
Branch 1 of the plan (§2.5) before continuing.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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
- [ ] 2.6 — **Audit extension (MT-2) — sweep beyond `performance_tests.rs`.** Specify each fix; do not edit the files in this phase beyond the MRC work above:
  - [`asd_tests.rs:42-49`](tests/asd_tests.rs#L42-L49) — `test_stability_no_flicker` asserts `config.general.update_ms >= 500`, the *metrics collector* period. The render tick is hard-coded 33 ms at [threads/mod.rs:116](src/core/threads/mod.rs#L116). C-05 is tested green against a clock production does not use. Retarget it at the tick, and at `target_fps` once Phase 5 introduces it.
  - [`asd_tests.rs:53-69`](tests/asd_tests.rs#L53-L69) — `test_layout_predictability` has **every assertion commented out** and iterates an empty body. It cannot fail. Restore the assertions or delete the test; a permanently-green test named for a requirement is worse than no test at all.
  - [`benches/render_bench.rs:17-32`](benches/render_bench.rs#L17-L32) — benches one `FontDescription` and one string through one layout: the same shape as the trap being removed. Label it a **control**, or point it at `RainManager::draw`.
- [ ] 2.6b — **S-13a — measure the Cairo-side per-frame cost outside the rain draw.** In the same MRC harness, at 4096×2160, time `clear()` (opaque full-surface paint), `rain.update`, and the metrics glow — the last being **six** `show_layout` calls per metric per frame ([drawing.rs:27-39](src/render/layout/drawing.rs#L27-L39): the `passes` loop *plus* a final full-alpha pass). Record `cairo_rest_ms`. **Do not time the present path here** — that is S-13b and it lives in Phase 1.7, because this harness has no X connection, no SHM segment and no RandR (round-2 audit).
- [ ] 2.7 — **Audit — make A-02 auditable going forward.** The 0.02 ms / 4.8 ms / 692→102 ms measurements behind the font-cache-eviction diagnosis exist only in a session transcript; no file records them. Write the MRC's first red run into the receipt: profile (`--release`), mean ms/frame, the full per-frame series, host CPU, geometry, realism. Do not fabricate a backdated investigation document — record forward from here.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-08 (first half): `cargo test --release --test performance_tests test_rain_frame_cost_mrc` **FAILS** with a mean **> 20 ms/frame**. **Audit correction:** the gate is "exceeds the threshold", not "reports ~750 ms" — that figure was measured under the dev profile and may not reproduce under `--release` (`opt-level = "z"`, LTO, `codegen-units = 1`). Record the actual figure under both profiles. A passing result here means the test is not exercising the production path — fix the test, not the threshold.
- [ ] **AC2** — The control test (identical glyph count, single font size) passes its gate under `--release`, confirming the delta is size-churn rather than glyph volume. **Round-2 audit:** do not assert the literal "~12 ms" — that figure was measured under the dev profile and carries the same problem AC1 just shed. Set the control's threshold from its own `--release` run and record both profiles.
- [ ] **AC3** — The 40-frame series shows **no warm-up convergence** (frame 40 within 15% of frame 1), confirming cache eviction rather than cold start.
- [ ] **AC4** — R-06: the MRC contains no synthetic glyph loop; it calls `RainManager::draw`.

- [ ] **AC5** — **Falsification checks X-1 and X-2 (plan §1.9). This AC decides whether Phase 3 opens at all.**
  - **X-1:** is the `--release` MRC mean **≤ 20 ms/frame**, on a test that still calls production `RainManager::draw` with *varying* sizes at 4096×2160 (R-06 must hold — confirm the test was not quietly simplified)? If yes, the ~750 ms baseline was a dev-profile artifact and Phase 3 has nothing to fix that could explain 61%.
  - **X-2:** are the MRC mean and the single-size control **within 20% of each other** under `--release`? If yes, A-02 is false *even if both are slow* — the cost is glyph volume or fill rate, not font-cache eviction, and neither bucketing nor the atlas buys the campaign anything.
  - Either one landing means **halt: F1 is the wrong root cause.** Do not open Phase 3. Keep Phases 1–2 — they stand on their own — and re-center on whatever S-13a/S-13b named. Record the decision and its numbers in the receipt either way, including when F1 survives.

### Forward contract to Phase 3 — **and the mandatory stop**
A red MRC exists that measures the real path. Phase 3 is complete when and only when it turns green.

**Execution halts here.** Phases 1 and 2 are the authorized scope. Before Phase 3 opens, evaluate
plan §1.9 against five measured readings — truthful `HUD CPU`, live `fps`, the `--release` MRC mean,
the control, and `present_ms` per CRTC — and re-derive Phase 5's default `target_fps` from them. The
10 fps in task 5.3 is a placeholder the budget identity is expected to lower.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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
- [ ] **AC1** — S-01 / S-08 (second half): `test_rain_frame_cost_mrc` **PASSES** at **< 20 ms/frame** under `--release`. **Round-3 audit: the absolute 20 ms figure is the gate.** The improvement ratio is descriptive and is computed against the `--release` baseline Phase 2 actually recorded — not against the dev-profile ~750 ms figure. Demanding "≥ 40× from 750 ms" would set an implicit 2 ms target if the release baseline turns out to be, say, 80 ms, making the AC unsatisfiable for reasons unrelated to the fix.
- [ ] **AC2** — S-10: `wc -l src/render/physics/*.rs` — every file ≤ **175** lines.
- [ ] **AC3** — `cargo test --release --lib --test performance_tests --test metrics_tests --test asd_tests` green; no clippy regressions. **Audit (R-11):** `tests/window_integration.rs` is excluded — it calls `create_all_windows` against the live `$DISPLAY` ([window_integration.rs:35,105,134](tests/window_integration.rs#L134)) and asserts 1920×1080 at (0,0) beneath a comment claiming `create_all_windows` hardcodes that size. It does not: [window/mod.rs:48-83](src/core/window/mod.rs#L48-L83) derives geometry from RandR, which on this host yields 4096×2160 + 1920×1080. The test cannot pass here and must not gate this phase; its fix is specified in Phase 2.6 and Phase 10.
- [ ] **AC4** — **R-01 user sign-off (blocking):** side-by-side screenshots, 6 buckets vs current, presented to the user. Z-depth must read as preserved. **Do not proceed to Phase 4 without an explicit answer.** On rejection, raise bucket count and re-present; on second rejection take Branch 2 (§2.5) and skip to Phase 4.
- [ ] **AC5** — Defect class *Mock Trap*: confirm AC1 passes because the renderer changed, not because the threshold moved. The Phase 2 test file must be unmodified in this phase — verify with `git diff --stat tests/`.

### Forward contract to Phase 4
Per-frame rain cost is bounded and measured. A bucket abstraction exists that the atlas will key on
— Phase 4 extends it rather than replacing it.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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
- [ ] **AC1** — S-02, **restated by audit**: do **not** tighten `test_rain_frame_cost_mrc`'s 20 ms gate. Phase 3 AC5 freezes that file as the proof its own fix worked; moving its threshold here means Phase 4 is green before any atlas exists if bucketing alone reaches 8 ms — and red for reasons unrelated to the atlas otherwise. Add instead a **structural** assertion: zero `pangocairo` `show_layout` calls on the rain path (the atlas blits via `set_source_surface` + `paint_with_alpha`). Record the observed ms/frame in the receipt as a metric, not as a gate.
- [ ] **AC2** — A-04 / R-02: measured atlas memory recorded in the receipt and under the declared cap.
- [ ] **AC3** — Startup time to first frame not regressed by more than 100 ms (lazy population working).
- [ ] **AC4** — S-10: all touched files ≤ 175 lines.
- [ ] **AC5** — Visual parity with Phase 3 at 16 buckets, or better.
- [ ] **AC6** — Fallback path exercised by a test that forces a rasterization failure.

### Forward contract to Phase 5
Per-frame render cost is bounded and small. The remaining CPU variable is *how often* frames are
drawn — Phase 5's domain.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

---

## Phase 5: Frame Governor

**STATUS: NOT STARTED** — LOE-3 · Resolves F4 · Aligns with [docs/pitfalls.md:72](docs/pitfalls.md)

### Objective
Fix the frame cap that fails open under load, and bring the refresh rate in line with the documented
refresh guidance: *"1Hz or 0.5Hz is sufficient. Avoid 60fps animations."* — [docs/pitfalls.md:72](docs/pitfalls.md).
**Citation corrected (round-2 audit):** this sentence is **not** in `concept.md` §IV, which states a
500 ms minimum update interval. Both documents bind; only the attribution was wrong.

### Tasks
- [ ] 5.1 — Fix `spawn_tick_thread` in [threads/mod.rs:114-125](src/core/threads/mod.rs#L114-L125). Time blocked in `send()` on the `bounded(1)` channel is currently counted in `elapsed`, so once a frame exceeds 33 ms the thread sleeps 1 ms and immediately re-queues — the cap disappears exactly when it is needed.
- [ ] 5.2 — Replace the sleep-accumulator with a **monotonic deadline** scheduler: compute the next tick instant from a fixed epoch, sleep until it, and *skip* missed ticks rather than queuing them. **R-03 / C-05:** this must not introduce visible stutter or strobing.
- [ ] 5.3 — Add `general.target_fps: u32` to [config/types.rs](src/core/config/types.rs) with `#[serde(default = "default_target_fps")]` (**C-02 — mandatory**) and a matching entry in [defaults.rs](src/core/config/defaults.rs). Default **10** *provisionally* — subject to AC6, which may force it lower. [docs/pitfalls.md:72](docs/pitfalls.md) sanctions 1Hz–0.5Hz explicitly, so a low default follows documented guidance rather than compromising it.
- [ ] 5.4 — **Theme 4:** clamp `target_fps` to `1..=60` on load. A zero value must not divide by zero; an absurd value must not re-create the runaway.
- [ ] 5.5 — Expose `target_fps` in the GUI General tab, following the existing widget/`update_config_from_widgets` pattern in [ui/gui/logic.rs](src/ui/gui/logic.rs).
- [ ] 5.6 — Unit test governor pacing: inject a simulated 200 ms frame; assert the next tick is not issued before the configured interval.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-07: with an injected 200 ms frame, the tick thread never re-queues faster than the configured interval. This is the direct regression test for F4.
- [ ] **AC2** — `target_fps` is honored: measured `fps` (Phase 1) tracks the configured value within ±10%.
- [ ] **AC3** — Clamping verified at boundaries: `0` → 1, `9999` → 60.
- [ ] **AC4** — C-02: existing config without `target_fps` loads and defaults correctly.
- [ ] **AC5** — **R-03 / C-05 user sign-off (blocking):** rain motion at the new rate must read as smooth and non-strobing. ASD guidance is a hard constraint, not a preference.
- [ ] **AC6** — **Audit — the budget identity gate (plan §1.3). Blocking.** Compute `(rain_ms + cairo_rest_ms + present_ms) × target_fps × monitors ÷ 10` at the chosen default, with `monitors = 2`. **Every term is a measured number by the time this phase runs** — `rain_ms` from the Phase 3/4 MRC, `cairo_rest_ms` from S-13a (task 2.6b), `present_ms` per CRTC from S-13b (task 1.7). The result must project **under 3%**. **Round-2 audit: the earlier "or a recorded estimate otherwise" clause is deleted** — a gate satisfiable by an unmeasured number is Hallucinated Success under a new name, and it is the exact defect class this audit exists to remove. Worked counter-example from the pre-audit draft: S-02's 8 ms ceiling × the proposed default of 10 fps = 8% of one core on the 4K panel alone, before `rest_ms` and before the second monitor — every written gate green, S-04 failed. If the projection exceeds 3%, **lower the default `target_fps` here** (plan §2.5 branch); do not defer the problem to the optional Phase 6.

### Forward contract to Phases 6 and 7
Frame cost (Phase 4) × frame rate (Phase 5) is now bounded and tunable, and AC6 has projected it under
S-04. **Phase 7 depends on this phase, not on Phase 6** *(audit correction to the plan §1.6 DAG)* —
Pulse Mode needs a governed interval and a `rain_mode` that survives startup, not damage tracking.
**Evaluate the §2.5 sequel here:** if S-04 already passes with margin, consult the user before
spending a day on Phase 6.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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
- [ ] 6.2 — Add a dirty flag to the metrics panel: re-render text only when the underlying `MetricValue` set changes, not every frame. **Audit — the scroll interaction:** [components.rs:53-57](src/render/layout/components.rs#L53-L57) advances a per-metric scroll offset by `+0.5` px on **every draw** when a value overflows its column. A value-change-only dirty flag freezes that scroll mid-string; a naive "always dirty while scrolling" cancels the glow-pass saving 6.5 claims. Resolve it explicitly: either treat an in-flight scroll offset as dirty for that item only, or disable scrolling while the panel cache is active. `concept.md` §IV and [pitfalls.md:73](docs/pitfalls.md) both prefer static text — the second option is the aligned default.
- [ ] 6.3 — Cache the rendered metrics panel to a separate Cairo surface; composite it over the rain each frame instead of re-running the glow passes.
- [ ] 6.4 — **R-04:** implement `force_full_redraw` and invoke it unconditionally on config change, `Expose`, theme change, and — subject to 6.4b — resize and monitor reconfiguration. Full repaint remains the fallback: damage tracking is an optimization layered over a correct path, never a replacement for one.
- [ ] 6.4b — **Audit (R-12) — "resize" is currently a fiction; pick one and write it down.** [`ShmPresenter::resize`](src/render/engine/presentation/shm.rs#L150-L152) is `Ok(())` — it does not reallocate the SHM segment — the socket path is the same, and [handlers.rs:14-41](src/core/threads/handlers.rs#L14-L41) handles only `KeyPress` and `Expose`, so no `RRNotify`/`ConfigureNotify` ever arrives to call it. Either **(a)** implement a real SHM teardown-and-rebuild plus a RandR event handler — note `shm.rs` is at **167 of the 175-line cap**, so this requires a **new module** under `presentation/` (C-01), not growth in place — or **(b)** drop "resize" and "monitor reconfiguration" from 6.4's trigger list and record the deferral in writing. Leaving a no-op behind a listed trigger is Ghost Logic.
- [ ] 6.5 — Verify the glow-pass reduction: [components.rs](src/render/layout/components.rs) draws 5 glow passes per metric per frame (from the live config's `glow_passes`). With caching these run only on change.
- [ ] 6.6 — Test: assert no stale-pixel artifacts across a config change and an expose event — and across a resize only if 6.4b took option (a). If it took option (b), assert instead that the deferral is documented and that no code path claims to handle resize.
- [ ] 6.7 — **Audit — opportunity, this phase only.** [`ShmPresenter::present`](src/render/engine/presentation/shm.rs#L114-L144) issues `CreateGc` + `FreeGc` on every frame on every monitor. A single `Gcontext` created once and held on the struct removes two X requests per frame per monitor. Take this **only if** Phase 6 proceeds and that file is already open; drop it silently if Phase 6 is abandoned per §2.5. It is not a mission item.

### Acceptance criteria (MRC)
- [ ] **AC1** — Metrics panel re-renders only on value change; verified by an instrumented counter over 100 frames at `update_ms: 2000` — expect ≈ 1 panel render per 2 s, not 1 per frame.
- [ ] **AC2** — **R-04:** no stale pixels after config change or expose — and after resize **only if 6.4b took option (a)**. If 6.4b took option (b), this AC instead requires that the deferral is written down and that no code path claims to handle resize. *(Round-2 audit: this AC previously demanded a resize guarantee unconditionally, contradicting 6.4b, which correctly makes resize handling a choice.)* This is the failure mode that makes damage rendering dangerous; it must be tested, not assumed.
- [ ] **AC3** — **R-05:** no mutex held across any Cairo call. Verify by inspection and by a test that renders while the metrics thread writes concurrently.
- [ ] **AC4** — Measured CPU improves or holds versus Phase 5. A regression here triggers revert to the Phase 5 commit (§1.7).
- [ ] **AC5** — S-10: all touched files ≤ 175 lines.

### Forward contract (Phase 6 is a leaf — it contracts to nothing)
The render loop does minimal per-frame work, and a static-content rendering path exists. **Phase 7
does not depend on this** *(round-2 audit: this heading still read "to Phase 7", contradicting the
rewired DAG)* — Pulse Mode's prerequisites are Phase 5's governed interval and the F8 fix, both
upstream of here. If Phase 6 is abandoned per §2.5, nothing downstream is blocked; the static-content
path it would have built is a convenience for Phase 7, not a requirement of it.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

---

## Phase 7: Pulse Mode

**STATUS: NOT STARTED** — LOE-4 · Resolves Ghost Logic, F8 · concept.md §II.1
**Depends on Phase 5, not Phase 6** *(audit correction)* — a governed interval and a surviving `rain_mode`, not damage tracking. Phase 6 may be abandoned without affecting this phase.

### Objective
Implement the mode `concept.md` §II.1 promises — *"static, pulsing glyphs for <0.5% CPU impact"* —
and which `test_pulse_mode_efficiency` has been asserting against for months. Today
[pipeline.rs:35](src/render/engine/pipeline.rs#L35) draws rain only when `rain_mode == "fall"`;
every other value silently draws **nothing**. The mode is not implemented, it is merely absent.

### Tasks
- [ ] 7.0 — **PREREQUISITE — F8. Blocking; nothing else in this phase can be verified before it.** [main.rs:28](src/core/main.rs#L28) executes `config.cosmetics.rain_mode = "fall".to_string();` unconditionally, immediately after `Config::load()` and with no comment. The user's configured mode is discarded on **every** launch. Remove the line, or gate it behind an explicit debug flag that defaults off. Until this is done, `pipeline.rs:35` can never see `"pulse"`, S-05 cannot be measured, and Phase 8's Minimal preset writes a value the next launch overwrites. Record why the line existed if `git log -S` reveals it; if not, say so.
- [ ] 7.1 — Implement `rain_mode: "pulse"` as a real branch: static glyph positions with a slow global alpha oscillation, no per-frame physics and no glyph churn.
- [ ] 7.2 — **C-05 (ASD):** the pulse must be a slow, smooth alpha ramp. *"No flashing or blinking elements"* — [docs/pitfalls.md:70](docs/pitfalls.md), **not** `concept.md` §IV (round-2 audit citation fix; §IV's contribution is the 500 ms minimum interval). Both bind. This constraint governs the implementation, not just the review.
- [ ] 7.3 — Reuse the Phase 4 glyph atlas — pulse mode blits the same cached surfaces at a varying alpha.
- [ ] 7.4 — Make the unhandled-`rain_mode` case explicit: an unknown value must log a warning and fall back to a known mode rather than silently rendering nothing. This is the Ghost Logic guard for the whole setting.
- [ ] 7.5 — Fix `test_pulse_mode_efficiency` (deferred from task 2.5) to exercise the now-real mode instead of an empty branch.
- [ ] 7.6 — Expose mode selection in the GUI Cosmetics tab.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-05 via **Method M-1** (plan §1.3): live process with `rain_mode: "pulse"` measures **< 0.5% of one core** over a 300 s window after a known restart. Two `/proc/<pid>/stat` samples, `pgrep -x matrix-overlay`.
- [ ] **AC1b** — **F8 closure, blocking AC1:** the configured `rain_mode` survives process start. Verify the **in-process** value (log the effective `config.cosmetics.rain_mode` after `Config::load()` and after any mutation), not merely the JSON on disk — the disk value was always correct; the substrate overwrote it in memory. An AC1 reading taken while the fall renderer is secretly active is Hallucinated Success.
- [ ] **AC2** — **R-09:** AC1 is a measured CPU reading with the mode visibly active on screen — not a code-exists check. The whole point of this phase is that the previous "implementation" was absent while a test passed.
- [ ] **AC3** — C-05: pulse period ≥ 2 s with a smooth ramp; no discontinuous alpha steps.
- [ ] **AC4** — An unknown `rain_mode` logs a warning and falls back, rather than rendering an empty screen.

### Forward contract to Phase 8
Multiple render modes with distinct, measured cost profiles exist — the raw material the
Performance Presets need in order to mean something.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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

  | Preset | `target_fps` | `realism` | `glow_passes` | `rain_mode` | S-04 obligation |
  |---|---|---|---|---|---|
  | Minimal | 1 | 2 | 1 | `pulse` | must meet **S-05** (< 0.5%) |
  | Medium *(default)* | see below | 4 | 3 | `fall` | **must meet S-04** (< 3%) |
  | Extreme | 30 | 10 | 5 | `fall` | **opt-in; exempt from S-04** |

  **Audit — the default is load-bearing.** `default_preset()` returns `"medium"` ([defaults.rs](src/core/config/defaults.rs)) and Phase 9.5 measures "the default preset", so Medium *is* the S-04
  configuration. Its `target_fps` is whatever Phase 5 AC6's budget identity permits — it is not a free
  choice, and the pre-audit value of 10 does not survive that identity against S-02's 8 ms ceiling.
  Extreme is explicitly a user opt-in to exceed the ambience budget; label it so in the GUI rather
  than letting it silently contradict `concept.md` §III.

- [ ] 8.3 — Make `perf_preset` authoritative on load: applying a preset writes the derived values into config and persists atomically via the existing `.tmp`-then-rename path in [config/storage.rs](src/core/config/storage.rs).
- [ ] 8.4 — Handle the divergence case explicitly: when individual settings are edited after a preset is applied, set `perf_preset` to `"custom"` rather than leaving a stale label claiming otherwise.
- [ ] 8.5 — Reflect the active preset in the GUI on open, so the displayed state matches the config.
- [ ] 8.6 — **§2.5 branch:** if the user decides the presets are unwanted, **remove** the three buttons and the `perf_preset` field entirely. Ghost Logic is resolved by implementation or by deletion — never by leaving it in place.
- [ ] 8.7 — **Audit (GL-2) — finish the Ghost Logic sweep `perf_preset` started.** The same rule applies to each: wire it or delete it, no third option.
  - `general.show_monitor_label` — declared [types.rs:20](src/core/config/types.rs#L20), defaulted [config/mod.rs:39](src/core/config/mod.rs#L39), **written** by the GUI at [logic.rs:21](src/ui/gui/logic.rs#L21) and [general.rs:59](src/ui/gui/general.rs#L59), and read by no layout or render code. A checkbox that saves a bit nothing consumes.
  - `logging.build_logging_enabled` — declared [types.rs:130](src/core/config/types.rs#L130), defaulted [types.rs:141](src/core/config/types.rs#L141), read nowhere.
  - `src/core/timer.rs` — declared at [core/mod.rs:13](src/core/mod.rs#L13); no `timer::` caller exists anywhere in `src/`. It is also the **only** caller of `factory::create_collectors` ([factory.rs:11](src/metrics/factory.rs#L11)), which is why that module appears live and is not. Deleting `timer.rs` orphans `factory.rs`; decide both together.
  - Verification for this task is the inverse of F5's: `grep -rn` each symbol and show a **read**, or show the declaration is gone.

### Acceptance criteria (MRC)
- [ ] **AC1** — S-09: clicking each preset changes `~/.config/matrix-overlay/config.json` on disk **and** produces a visible render change. Both halves required — a config write with no render effect is the same Ghost Logic in a new location.
- [ ] **AC2** — Measured CPU (Method M-1) differs materially between Minimal and Extreme, confirming the presets drive real cost. Minimal must satisfy S-05; Medium must satisfy S-04; Extreme is recorded but not gated.
- [ ] **AC3** — Editing an individual setting after applying a preset sets `perf_preset: "custom"`.
- [ ] **AC4** — C-02: config round-trips through save/load without loss.
- [ ] **AC5** — `grep -rn "perf_preset" src/` shows the field is **read**, not merely declared — the direct inverse of the F5 finding.

### Forward contract to Phase 9
A feature-complete binary satisfying S-01, S-02, S-03, S-05, S-06, S-07, S-09.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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
- [ ] 9.5 — Measure the live process with **Method M-1** (plan §1.3) over a 300 s window at the default preset — *after* the restart in 9.4, so the window reflects the new binary rather than a lifetime average that includes the old one.
- [ ] 9.6 — Measure again with `rain_mode: "pulse"`, having confirmed via 7.0/AC1b that the mode is actually in effect in-process.
- [ ] 9.6b — **S-13 confirmation, not first contact** *(round-2 audit)*. `cairo_rest_ms` was measured in Phase 2.6b and `present_ms` per CRTC in Phase 1.7; both already gated Phase 5 AC6. Here, re-measure them on the **deployed** binary and confirm they match the earlier figures within a recorded tolerance. A divergence between the harness numbers and the live ones is itself the diagnosis AC7 needs — it says the gap is environmental, not architectural.
- [ ] 9.7 — Cross-check the on-screen `HUD CPU` against **Method M-1**, and the on-screen `fps` against a 10 s wall-clock count of `Presenter::present` calls — closing the loop on the two instruments repaired in Phase 1. *(Round-2 audit: this task previously said "against `ps`", the instrument M-1 replaces; and `ps` cannot measure frame rate at all.)*
- [ ] 9.8 — Record all readings in the receipt, including any criterion that **failed**.

### Acceptance criteria (MRC)
- [ ] **AC1** — **S-04 — THE MISSION GATE:** live process **< 3% of one core** over a 300 s window, **via Method M-1** (plan §1.3), taken after the 9.4 restart. Baseline for comparison: **60.7%** (pid 2462, measured over a 25 h lifetime). **Audit correction:** the previous command, `ps -o pcpu= -p $(pgrep -f matrix-overlay)`, cannot certify this. `ps` `%CPU` is `100 × cputime ÷ lifetime` — a lifetime average, so a correctly fixed process still prints ~60% for hours after restart-free deployment — and `pgrep -f` additionally matches cargo and repository paths, returning multiple pids. Record the raw `utime`/`stime` tick deltas alongside the percentage.
- [ ] **AC2** — S-05: `< 0.5%` in Pulse Mode, same method, with AC1b's in-process confirmation that Pulse is actually active.
- [ ] **AC3** — S-12, **restated by audit**: after `scripts/install.sh`, `cmp ~/.local/bin/matrix-overlay target/release/matrix-overlay` must report identical files, and the built git sha is recorded in the receipt. The previous check — `stat` mtime newer than `git log -1 --format=%ci` — proves only that some file is newer than some commit date; it cannot show that commit `380107f` (the SHM `pre_draw` fix) is present in the installed bits, which is the entire content of F3. mtime remains a secondary sanity check.
- [ ] **AC4** — S-03 confirmed live: on-screen `HUD CPU` within ±1pp of Method M-1.
- [ ] **AC5** — No visual regression versus the pre-remediation overlay; user confirms.
- [ ] **AC6** — Defect class *Hallucinated Success* / *Sound Effect Execution*: measurements are taken from the **live deployed process**, never from a test harness or a dev build.
- [ ] **AC7** — **§2.5 halt condition:** if AC1 fails while the MRC is green, **halt the campaign and do not document success.** A gap between MRC-green and live-red proves a cost centre outside the rain path.

  **Check the operational suspects first — they explain an *insane* reading, not a merely high one:**
  1. Method M-1 attached to the wrong pid — a Phase 1 `cargo run` still alive, or a `pgrep` that returned two.
  2. `target_fps` not actually applied (Ghost Logic repeat — still the 33 ms tick).
  3. F8 still in place, so what was measured as "Pulse" was the fall renderer.

  **Then the real suspects, ranked by *cost the MRC never saw, paid on both CRTCs* (round-2 audit — this list replaces the earlier unranked one):**
  1. **`Presenter::present` × 2 monitors.** `GetInputFocus` round-trip plus `ShmPutImage` of a 4096×2160×4 buffer (≈35 MB), then again for 1920×1080 (≈8 MB). Absent from the MRC entirely. This is the floor S-13b exists to name — **check it first**, and compare against the `present_ms` recorded in Phase 1.7.
  2. **Metrics glow.** [drawing.rs:27-39](src/render/layout/drawing.rs#L27-L39) — six `show_layout` calls per metric per frame, every tick, both windows. Also absent from the MRC. Once F1 is dead this is the next Pango cost centre; compare against `cairo_rest_ms` from Phase 2.6b.
  3. **Opaque full-surface `clear()` × 2.** A 35 MB write on HDMI-1-0 plus 8 MB on eDP, every frame, before a single glyph is drawn.
  4. **`rain.update` running when `draw` does not.** [pipeline.rs:33](src/render/engine/pipeline.rs#L33) is **not** behind the `"fall"` gate at line 35 — physics advances even in modes that render nothing. Cheap relative to present, free to check, and it is the Pulse-mode leak.
  5. **`CreateGc`/`FreeGc` per present, then the `SharedMetrics` lock held across both monitors.** Two X requests and a lock are not a 3-percentage-point miss on their own. Look here only after 1–4 have numbers.

### Forward contract to Phase 10
Verified live measurements exist to document — including any failures.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

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
- [ ] 10.6 — Update CLAUDE.md's module map for `glyph_cache.rs`. **Audit:** in the same pass correct [CLAUDE.md:97](CLAUDE.md#L97) and [CLAUDE.md:119](CLAUDE.md#L119), which both name a flat `presentation.rs`. That file has not existed since commit `e948079`; the Presenter lives at `src/render/engine/presentation/{mod,shm,socket}.rs`. *(The audit's claim that this stale path also appears in the plan documents did not survive verification: at audit time `grep -n presentation implementation-plan.md tasks.md` returned no hits. Only CLAUDE.md is wrong.)*
- [ ] 10.7 — **Audit — citation corrections.** `concept.md` §IV does **not** contain the "1Hz or 0.5Hz is sufficient" guidance or the phrase "No flashing or blinking elements"; both are [docs/pitfalls.md:70-72](docs/pitfalls.md). `concept.md` §IV states a **500 ms minimum update interval**. Both documents are binding and the policy is unchanged, but the plan's §1.1 and C-05 cited the wrong one, and that misattribution should not propagate into pitfalls.md or CLAUDE.md.
- [ ] 10.8 — **Audit — record the test-suite findings.** Add to pitfalls.md, alongside the Mock Trap entry from 10.4: `test_stability_no_flicker` guarding C-05 against `update_ms` while the renderer ticks at 33 ms; `test_layout_predictability` shipping with every assertion commented out; and `tests/window_integration.rs` mapping real desktop windows onto `$DISPLAY` while asserting a hardcoded 1920×1080 that RandR contradicts (R-11). The rule worth recording is the general one: **a test that cannot fail, and a test that measures a clock production does not use, are both Mock Traps.**

### Acceptance criteria (MRC)
- [ ] **AC1** — pitfalls.md contains both new entries with measured numbers, matching the existing entry format.
- [ ] **AC2** — CLAUDE.md's clear-to-transparent statement is corrected and consistent with pitfalls.md.
- [ ] **AC3** — Every new config field is documented.
- [ ] **AC4** — A fresh reader can reconstruct why 147 distinct font sizes per frame cost 750 ms, from the documentation alone.
- [ ] **AC5** — The campaign's final measured numbers are recorded in DevJournal.md, including any unmet criterion.

**Receipt:** `receipts/BUILD_RECEIPTS.md` (repo-root relative; created on first write by Phase 1)

---

## Phase Summary

| Phase | LOE | Depends on | Resolves | Gate | Risk |
|---|---|---|---|---|---|
| 1 — Instrumentation Truth | 1 | — | F2, F6 | S-03, S-06, **S-13b**, **§1.9 X-3** | Low |
| 2 — Disarm the Mock Trap | 1 | 1 | MT, MT-2 | S-08 (red), **S-13a**, **§1.9 X-1/X-2** | Low |
| 3 — Font Size Bucketing | 2 | 2 | F1 core | S-01 (< 20 ms) | **R-01**, R-11 |
| 4 — Glyph Atlas | 2 | 3 | F1 complete | S-02 (0 hot-path shapes) | R-02 |
| 5 — Frame Governor | 3 | 4 | F4 | S-07, **AC6 budget** | **R-03** |
| 6 — Damage + Mutex | 3 | 5 · *optional* | architecture | AC1–AC5 | **R-04, R-05**, R-12 |
| 7 — Pulse Mode | 4 | **5** *(not 6)* | Ghost Logic, **F8** | S-05 (< 0.5%) | R-09 |
| 8 — Wire Presets | 4 | 7 | F5, F7, **GL-2** | S-09 | Low |
| 9 — Deploy + Verify | 5 | 8 | F3, **F9** | **S-04 — MISSION**, S-12, S-13 confirm | R-08 |
| 10 — Documentation | 5 | 9 | recurrence | AC1–AC5 | Low |

**Blocking user gates:** Phase 3 AC4 (Z-depth), Phase 5 AC5 (motion smoothness), Phase 9 (restart approval).

**Blocking internal gates (added by audit):** Phase 5 AC6 (budget identity must project under 3% before
the phase closes), Phase 7 AC1b (F8 must be fixed or S-05 is unmeasurable).

**Abandonable:** Phase 6, per §2.5 sequel, if S-04 already passes with margin after Phase 5. Phase 7 no
longer depends on it.

**Halt conditions:** **Phase 2/3 stop — plan §1.9.** If X-1, X-2 or X-3 lands, F1 is the wrong root
cause: halt, keep Phases 1–2, re-center. Phase 9 AC7 — live measurement failing while the MRC is green means a second cost
centre exists; halt, consult the S-13 suspect list, do not document success. Phase 7 — reached with F8
unfixed; halt rather than measure Pulse Mode against the fall renderer.

---

## Audit Trail — 2026-09-03

This task list was revised by an adversarial audit performed before any phase executed. Every finding
was verified against the substrate first; three did not survive and were rejected. The accept/reject
ledger, with verifying commands, is at the foot of [implementation-plan.md](implementation-plan.md).

Nothing in this document has been implemented. No source file, test, `CLAUDE.md`, or `docs/pitfalls.md`
was modified by the audit pass — those changes belong to the phases above.
