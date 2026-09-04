**IMPLEMENTATION PLAN — RENDER SUBSTRATE REMEDIATION CAMPAIGN**

**Plan ID:** `20260903-matrixoverlay-render-remediation`
**Created:** 2026-09-03
**Selected Option:** F — Full Concept Realization (High-Fidelity Tier)
**Source:** 2026-09-03 CPU investigation session (findings F1–F7)
**Companion:** [tasks.md](tasks.md) — the executable phase breakdown

---

## [INTENT] User Objective

> **Make the Matrix Overlay stop eating 60% of a CPU core, and make it honest about what it costs.**
>
> The overlay was discovered pegged at a sustained 60.7% of one core for its entire ~23.5-hour
> runtime — the single largest CPU consumer on the machine, ahead of Firefox and gnome-shell —
> while its own on-screen "Overlay CPU" readout said "4% ish." The user's words: *"supposed to be
> a lot less than that."*
>
> `concept.md` §III states the target plainly: **"Total system impact target is <1-3% CPU usage."**
> The overlay is running 20–60× over its own documented budget. This is not a tuning exercise;
> it is a correctness failure against a stated contract.
>
> The **why**: this is a *desktop ambience* application. It exists to sit quietly behind real work
> and display a pulse of the system. An ambience layer that consumes more CPU than the browser
> has inverted its own purpose. Worse, the instrument that was supposed to reveal this — the
> overlay's own `overlay_cpu` metric — under-reported the fault by exactly 16×, which is why it
> ran unnoticed for a day.
>
> Option F was selected: fix the defect, fix the instrument that hid it, fix the test that stood
> guard over it, and finish the render architecture the concept document already describes but
> the code never implemented.
>
> **Marked `/nodelete`.** This section may never be removed, only updated by explicit user
> instruction. All future `/focus-plan` runs compare the substrate against this statement.

---

# PART 1 — UNIVERSAL STRUCTURAL

## 1.1 Confirmed User Intent & Concept

Remediate all seven evidence-backed defects from the 2026-09-03 investigation as **one coordinated
campaign**, not seven independent patches. The user explicitly selected **Option F** — the
high-fidelity tier — after reviewing six options at the HITL gate.

Option F is not merely "fix the bug." It is: *bring the render substrate into alignment with what
`concept.md` already promises*. Three promises in that document are currently unfulfilled in code:

| Documented promise | Source | Current substrate reality |
|---|---|---|
| "Total system impact target is **<1-3% CPU**" | `concept.md` §III | 60.7% of one core, sustained |
| "**Pulse Mode**… static, pulsing glyphs for **<0.5% CPU** impact" | `concept.md` §II.1 | Not implemented. `pipeline.rs:35` draws nothing for any `rain_mode != "fall"` — and `main.rs:28` overwrites `rain_mode` at startup regardless, so the branch is unreachable (**F8**) |
| "**1Hz or 0.5Hz is sufficient.** Avoid 60fps animations" | [docs/pitfalls.md:72](docs/pitfalls.md) — **not** `concept.md` §IV | Hard-coded 33ms (30fps) tick, with a cap that fails open under load |

**Citation correction (2026-09-03 audit).** The 1Hz/0.5Hz refresh guidance and the phrase "No flashing
or blinking elements" were previously attributed to `concept.md` §IV. Both live in
[docs/pitfalls.md:70-72](docs/pitfalls.md). `concept.md` §IV states a **500 ms minimum update interval**
to prevent strobing. Both documents remain binding and the policy is unchanged; only the citation was
wrong. Corrected here, in §1.4 C-05, and in `tasks.md` Phase 10.

The campaign closes all three.

## 1.2 Scope & Boundaries

### In scope

| ID | Defect | Location |
|---|---|---|
| **F1** | Font-cache eviction in rain renderer — the root cause | [rain_manager.rs:38-42](src/render/physics/rain_manager.rs#L38-L42) |
| **F2** | `overlay_cpu` double-normalized by core count (16×) | [process.rs:27-30](src/metrics/collectors/system/process.rs#L27-L30) |
| **F3** | Deployed binary predates the SHM `pre_draw` sync fix | `~/.local/bin/matrix-overlay` vs commit `380107f` |
| **F4** | 30fps cap fails open — blocked `send()` counted in `elapsed` | [threads/mod.rs:114-125](src/core/threads/mod.rs#L114-L125) |
| **F5** | `perf_preset` config field read nowhere | [types.rs:98](src/core/config/types.rs#L98) |
| **F6** | Live frame rate not directly observable (inferred only) | — (new capability) |
| **F7** | Performance Preset buttons connected to nothing | [advanced.rs:12-19](src/ui/gui/advanced.rs#L12-L19) + [mod.rs:79](src/ui/gui/mod.rs#L79) |
| **MT** | Mock Trap: perf test validates the path production doesn't take | [performance_tests.rs:61-87](tests/performance_tests.rs#L61-L87) |
| **F8** | `rain_mode` clobbered at startup — user's configured mode is discarded on every launch | [main.rs:28](src/core/main.rs#L28) |
| **F9** | Per-frame cost outside `RainManager::draw` is unmeasured (clear, glow, present, ×2 monitors) | [pipeline.rs:28-38](src/render/engine/pipeline.rs#L28-L38), [shm.rs:114-146](src/render/engine/presentation/shm.rs#L114-L146) |
| **MT-2** | Mock Traps outside `performance_tests.rs` — ASD flicker test, empty predictability test, Criterion bench | [asd_tests.rs:42-69](tests/asd_tests.rs#L42-L69), [render_bench.rs:17-32](benches/render_bench.rs#L17-L32) |
| **MT-3** | `tests/metrics_tests.rs` has never compiled — calls a nonexistent `NvidiaSmiCollector::new_with_command`; zero coverage for an unknown period | [metrics_tests.rs](tests/metrics_tests.rs), [nvidia.rs:11](src/metrics/collectors/nvidia.rs#L11) |
| **GL-2** | Ghost Logic beyond `perf_preset`: `show_monitor_label`, `build_logging_enabled`, dead `core/timer.rs` | [types.rs:20](src/core/config/types.rs#L20), [types.rs:130](src/core/config/types.rs#L130), [timer.rs](src/core/timer.rs) |

Plus Option F architecture: glyph atlas, frame governor, damage-based rendering, `SharedMetrics`
mutex removal from the render path, frame-time histogram, and Pulse Mode implementation.

### Explicitly out of scope

- **The opaque-black `clear()`.** Investigated and found **correct**. [pitfalls.md](docs/pitfalls.md)
  documents `Operator::Source` + `rgba(0,0,0,1.0)` as the CONFIRMED fix from 2026-05-21 for a prior
  regression. The defect is in [CLAUDE.md](CLAUDE.md)'s prose ("Clear… to transparent"), corrected
  in Phase 10 as documentation only. **No code change.**
- Metric collector logic beyond `overlay_cpu` and the new `fps` collector (CPU/RAM/GPU/weather/git
  collectors untouched). **Audit amendment:** [manager.rs:54](src/metrics/manager.rs#L54) replaces
  `SharedMetrics.data` wholesale each `update_ms` (`sh.data = MetricData { values: frame }`), so a
  renderer-published `fps` value would be erased on the next collection tick. S-06 therefore requires
  a minimal, named touch to `manager.rs`/`dispatch.rs` — the collector *logic* stays out of scope, the
  registration of one new collector does not.
- Config schema migration. New fields use `#[serde(default)]` per CLAUDE.md; existing configs
  must continue to parse unchanged.
- The `productivity` / Ollama / update-checker threads.
- Wayland support. X11 only, as today.

### Boundary rule

**`s.depth` must remain continuous.** `concept.md` §II.1 requires *"proportional speeds based on
apparent Z-depth (size/brightness/speed correlation)."* Only the **font size** derived from depth
may be quantized. Quantizing `depth` itself would flatten the Z-depth illusion — a regression
against stated intent dressed up as a performance fix. See §1.5 R-01.

## 1.3 Success Criteria (measurable)

Every criterion below is a command or a reading, not a judgment.

| # | Criterion | Measurement | Gate |
|---|---|---|---|
| **S-01** ⚠️ **LAB_F1 — no longer a campaign gate (round-8)** | Rain frame cost collapses | MRC test (§1.8), `cargo test --release`, against production `RainManager::draw` | **< 20 ms/frame** at 4096×2160, realism=4. **The cargo-test MRC is a lab reproduction of F1, not a measurement of the live path** — it costs 438.66 µs/glyph where the overlay process costs 7.42 µs/glyph on identical volume. The live figure is **9.6220 ms**, already inside this threshold. S-01 is retained as documentation; it gates nothing. *Round-3: the absolute gate is the binding one. The former "≥ 40× faster than baseline" is descriptive only and is computed against the **measured `--release` baseline** recorded in Phase 2, never against the dev-profile ~750 ms figure — if the release baseline is 80 ms, "40×" would demand 2 ms and be unsatisfiable by construction.* |
| **S-02** | Glyph atlas removes Pango from hot path | New assertion, post-Phase 4: zero `show_layout` calls on the rain path (atlas blit only). Observed ms recorded in the receipt, **not** a moved threshold on the S-01 test | Hot-path Pango shaping count = **0** |
| **S-03** | `overlay_cpu` matches an external CPU reading | **Method M-1** (below) vs the on-screen `overlay_cpu` value | Within **±1 percentage point** |
| **S-04** | Live process meets concept.md §III | **Method M-1** on the deployed binary, 300 s window, after a known restart | **< 3% of one core** |
| **S-05** | Pulse Mode meets concept.md §II.1 | **Method M-1**, with `rain_mode: "pulse"` confirmed live in-process (F8) | **< 0.5% of one core** |
| **S-06** | Frame rate is directly readable | On-screen `fps` metric vs an independent 10 s wall-clock count of `Presenter::present` calls | Reported fps within **±10%** of the wall-clock count |
| **S-07** | Frame cap holds under load | Governor test: inject a 200 ms frame, observe pacing | Tick never re-queues faster than the configured interval |
| **S-08** ⚠️ **VACATED (round-8)** | Mock Trap disarmed | `cargo test --release --test performance_tests` | ~~The replacement MRC **fails before** Phase 3 and **passes after**~~ — **red-before/green-after is vacated because Phase 3 is not opening.** The Mock Trap *was* disarmed (`test_render_optimization_bench` deleted, R-06 rule written, control labeled) and that half stands. The transition proof presumed a Phase 3 that the in-process 1.25× ratio has demoted. |
| **S-09** | Presets are real | Click Minimal / Medium / Extreme in the GUI | Config changes on disk **and** render behavior changes |
| **S-10** | Module line limits honored | `wc -l` on every touched file | ≤ **175** lines (concept.md §III) |
| **S-11** | No config regression | Load the user's existing `config.json` unmodified | Parses without error; `deny_unknown_fields` satisfied |
| **S-12** | Deployed binary contains the campaign's work | `cmp ~/.local/bin/matrix-overlay target/release/matrix-overlay` after `install.sh`; record the built git sha | Byte-identical to the just-built release binary |
| **S-13a** | Cairo-side per-frame cost outside `RainManager::draw` is measured — `clear()`, `rain.update`, metrics glow | Instrumented at 4096×2160 inside the **Phase 2** MRC harness | `cairo_rest_ms` recorded; feeds the budget identity below |
| **S-13b** | X-side per-frame cost is measured — `pre_draw`'s `GetInputFocus` round-trip, `ShmPutImage`, `CreateGc`/`FreeGc`, **per monitor** | Instrumented on the **Phase 1** temporary run: `cargo build --release`, then `./target/release/matrix-overlay` executed **directly** (never via `cargo run` — see the pid rule in Phase 1 AC2), against a real X connection and real RandR geometry | `present_ms` recorded **per CRTC**; feeds the budget identity below |

**S-04 is the campaign's definition of done.** Everything else is a means to it.

### Method M-1 — the one live-CPU measurement (binding on S-03, S-04, S-05)

`ps -o pcpu` reports `100 × cputime ÷ process lifetime` — a lifetime average, not a window. A process
that ran 23.5 h at 61% still prints ~60% for hours after a successful fix. `top` reports an
instantaneous sample and is a *different instrument*; the two were used interchangeably in earlier
drafts. Neither measures "5-minute steady state". Every live-CPU criterion uses exactly this:

```bash
PID=$(pgrep -x matrix-overlay)              # -x, not -f: -f matches cargo and repo paths
[ "$(echo "$PID" | wc -w)" -eq 1 ] || exit 1   # exactly one pid, or the reading is meaningless
read_ticks() { awk '{print $14+$15}' /proc/$PID/stat; }   # utime + stime
T0=$(read_ticks); sleep 300; T1=$(read_ticks)
echo "scale=2; 100 * (($T1 - $T0) / $(getconf CLK_TCK)) / 300" | bc   # % of one core
```

The process must have been **restarted** before T0 for the window to reflect current code. Record the
raw tick values, not only the percentage.

### Budget identity (binding on Phase 5 and Phase 8)

```
cpu_pct_of_one_core  ≈  (rain_ms + cairo_rest_ms) × fps × monitors ÷ 10      <- per-surface work
                      + (present_ms_hdmi + present_ms_edp) × fps ÷ 10        <- already per-CRTC
```

**Round-4 correction — the single-line form double-counted present.** `cairo_rest_ms` (S-13a) is
measured once, on one surface in the MRC harness, so it scales by `monitors`. `present_ms` (S-13b) is
measured **separately for each CRTC** and the two are already summed, so scaling that sum by `monitors`
again counts every present twice. X-3 in §1.9 already uses the second line alone; Phase 5 AC6 must use
both. `monitors` is **2** on this host
(4096×2160 HDMI-1-0 + 1920×1080 eDP, both driven by
[window/mod.rs:48-83](src/core/window/mod.rs#L48-L83) via RandR).

**`rest_ms` is two different clocks and must be measured in two different places** *(round-2 audit)*.
The Cairo half (S-13a) needs a production-geometry surface, which the Phase 2 MRC harness builds. The
X half (S-13b) needs a live X connection, SHM attachment and real RandR output — none of which Phase 2
stands up. **Timing the present path inside the MRC harness would be a new Mock Trap**: a present-path
number taken off a path production does not take. S-13b therefore rides the temporary release run
that Phase 1 already starts for AC2/AC3, where the connection is real — `cargo build --release` as a
**separate, completed step**, then `./target/release/matrix-overlay` executed directly. Never
`cargo run`; see the pid rule in Phase 1 AC2.

Both halves are **measured before Phase 5 closes**. Phase 5 AC6 carries no estimate escape hatch.

### Round-8 correction — the identity was missing a third term

Phase 5 measured M-1 at **3.0166%** with `target_fps = 1` where the two-line identity projected
**2.06%**. The render half was right: **1.943%** measured against a 20.64 ms/tick projection, inside
6%. What the identity had **no term for** is a floor of **~1.07%** that does not scale with `fps` at
all — the metrics collectors on the 2 s `update_ms` cycle (the `nvidia-smi` subprocess among them),
the GTK/tray thread, and the XCB event thread.

```
cpu_pct ≈ (rain_ms + cairo_rest_ms) × fps × monitors ÷ 10
        + (present_ms_hdmi + present_ms_edp) × fps ÷ 10
        + floor_pct                    <- frame-rate INDEPENDENT; measured 1.07% on this host
```

**Lowering `target_fps` cannot reduce `floor_pct`.** Any projection that omits it promises a number
the process cannot reach, and at a 3% gate a 1% floor is a third of the entire budget. It must be
measured — M-1 minus the summed render terms at a known rate — never assumed.

**This identity is why S-01…S-08 can all pass while S-04 fails.** Worked example with the *written*
gates: S-02's 8 ms ceiling × Phase 5's proposed default of 10 fps = 80 ms/s = **8% of one core for the
4K rain draw alone** — before `rest_ms`, before the second monitor. Any phase that sets a frame rate
must evaluate this identity before closing.

## 1.4 Constraints & Assumptions

### Hard constraints

- **C-01 — Line limits.** 125-line target, **175 hard max** per module (`concept.md` §III).
  `rain_manager.rs` is at 63 today. The atlas requires a **new module**, not growth in place.
- **C-02 — Config compatibility.** `#[serde(deny_unknown_fields)]` is on every config struct.
  Every new field **must** carry `#[serde(default)]` or the user's live config fails to parse
  (CLAUDE.md, Configuration).
- **C-03 — Z-depth fidelity.** Per §1.2 boundary rule.
- **C-04 — X11 / single deploy path.** `scripts/install.sh` is the only supported deploy route.
- **C-05 — ASD design guidance.** `concept.md` §IV (high contrast; **500 ms minimum update interval**;
  no floating or scrolling distractions) together with [docs/pitfalls.md:70-72](docs/pitfalls.md)
  (**"No flashing or blinking elements"**; 1Hz–0.5Hz refresh sufficient; static text preferred over
  scrolling). Both are binding. The frame governor must not introduce visible stutter or strobing.
- **C-06 — Live restart requires approval.** Phase 9 restarts the user's running desktop overlay.
  That is a user-visible action and is gated on explicit approval at that phase.

### Assumptions (stated, falsifiable)

- **A-01** — The ~1.3 fps live frame rate is **inferred**, not measured (F6). ptrace_scope=1 blocked
  direct instrumentation and Xvfb is absent. **Phase 1 converts this assumption into a measurement.**
  If Phase 1 shows the live rate is *not* ~1.3 fps, the frame-budget arithmetic in Phases 5–6 must be
  re-derived (§2.5 Branch 1). **F1 does not stand unconditionally on that account** *(round-4/5
  correction)* — a wrong A-01 says nothing about whether font-cache eviction is present. In the
  `fps ∈ (2, 15)` band F1 stands only if §1.9's X-1 and X-2 both miss; at `fps ≥ 15` with the present
  budget above 40%, X-3 fires and F1 is falsified.
- **A-02** — Pango font-cache eviction is the mechanism. Evidence: 0.02 ms for 239
  `set_font_description` calls in isolation, versus ~4.8 ms per size change when a `show_layout`
  follows, versus a drop from 692 ms → 102 ms on the identical workload when live layout references
  are held. Falsifiable by the Phase 2 MRC — and **§1.9 now states exactly which readings falsify it**,
  so a disappointing result cannot be absorbed as "needs re-deriving".
- **A-03** — 6–16 size buckets preserve perceived Z-depth. **Subjective; user-verified at Phase 3.**
- **A-04** — Glyph set is bounded (Katakana `0x30A1..=0x30F6` = 86 glyphs). At 16 buckets the atlas
  holds ≤ 1,376 small surfaces. Assumed to fit comfortably in memory; measured at Phase 4.

## 1.5 Risk Assessment & Mitigation

| ID | Risk | Sev | Mitigation |
|---|---|:--:|---|
| **R-01** | Bucketing flattens Z-depth → visual regression sold as a perf win | **High** | Quantize *font size only*; `depth` stays continuous for alpha and speed. Side-by-side screenshot review with the user at Phase 3 before proceeding. |
| **R-02** | Atlas memory blowup on large fonts / many buckets | Med | A-04 measured at Phase 4; hard cap on bucket count; LRU eviction if the cap is exceeded. |
| **R-03** | Frame governor introduces visible stutter — violates C-05 (ASD) | **High** | Governor paces on a monotonic deadline, never a sleep-accumulator. User visual sign-off at Phase 5. |
| **R-04** | Damage-based rendering leaves stale pixels on screen | **High** | Full-surface repaint remains the fallback; damage tracking is an optimization with an explicit `force_full_redraw` escape hatch on any config change, expose event, or resize. |
| **R-05** | Removing the `SharedMetrics` mutex introduces a data race | **High** | Snapshot-and-release (clone the metric map, drop the guard, then render). No lock held across Cairo. |
| **R-06** | The replacement MRC becomes a *new* Mock Trap | **High** | The MRC must call production `RainManager::draw` with **varying** sizes at real geometry — never a synthetic loop. Its red-before / green-after transition (S-08) is itself the proof it measures the real path. |
| **R-07** | New config fields break the user's live config | Med | C-02 enforced per field; S-11 loads the actual user config as a test fixture. |
| **R-08** | Deploying a regression to a daily-driver desktop | Med | Phase 9 gated on approval; rollback is a single `cp` of the retained prior binary (§1.7). |
| **R-09** | Pulse Mode ships as another empty branch (Ghost Logic repeat) | Med | S-05 is a measured CPU reading with the mode active, not a code-exists check. |
| **R-10** | Phase 6 architecture work destabilizes a working fix from Phase 3–4 | Med | Phases 3, 4, 5 each end at a shippable, independently verified state. Phase 6 can be abandoned without losing the CPU win. |
| **R-11** | `cargo test` maps real `_NET_WM_WINDOW_TYPE_DESKTOP` windows onto the user's live desktop, and `test_geometry_and_visual` asserts 1920×1080 at (0,0) — false on this host, where RandR yields 4096×2160 + 1920×1080. Any AC reading "`cargo test` fully green" is unsatisfiable here. | **High** | `tests/window_integration.rs` runs only under Xephyr/Xvfb, or is `#[ignore]`d for campaign gates. No phase gate may depend on its geometry assert. The false comment at [window_integration.rs:143](tests/window_integration.rs#L143) ("hardcoded 1920x1080 in create_all_windows") is corrected in Phase 10. |
| **R-12** | `Presenter::resize` is a no-op on both paths ([shm.rs:150-152](src/render/engine/presentation/shm.rs#L150-L152)) and no `RRNotify`/`ConfigureNotify` handler exists ([handlers.rs:14-41](src/core/threads/handlers.rs#L14-L41)), so Phase 6.4's `force_full_redraw`-on-resize cannot reallocate the 4K SHM segment | Med | Phase 6 either implements a real SHM rebuild — in a **new module**, since `shm.rs` is at 167 of 175 lines — or drops "resize" from 6.4's trigger list and defers RandR reconfiguration explicitly, in writing. Silent no-op is not an option. |

## 1.6 Dependencies

### Sequencing (forward contracts — see Part 2 §2.4)

```
Phase 1 (instruments)  ─┬─→ Phase 2 (MRC, red)  ──→ Phase 3 (bucketing, green)  ──→ Phase 4 (atlas)
                        │                                                              │
                        └──────────────────────────────────────────────────────────────┤
                                                                                       ▼
                                                            Phase 5 (governor + tick fix)
                                                                       │
                                        ┌──────────────────────────────┼──────────────────────────────┐
                                        ▼                              ▼                              │
                          Phase 6 (damage + mutex)          Phase 7 (Pulse Mode)                       │
                            OPTIONAL — §2.5 sequel           requires F8 fix (main.rs:28)              │
                                        │                              │                              │
                                        └──────────────┬───────────────┘──────────────────────────────┘
                                                       ▼
                                          Phase 8 (presets)  ──→  Phase 9 (deploy)  ──→  Phase 10 (docs)
```

**Phase 7 depends on Phase 5, not Phase 6** *(audit correction)*. Pulse Mode needs a governed frame
interval and a `rain_mode` value that survives startup — not damage tracking. Parking S-05 behind the
one phase §2.5 already declares abandonable would make the concept contract hostage to optional work.
Phase 6 remains optional and parallel.

**Phase 1 must precede everything.** F2 is the instrument; verifying a CPU fix against a gauge that
reads 16× low is the exact trap that let this defect live for a day.

**Phase 2 must precede Phase 3.** A test written *after* the fix cannot demonstrate it caught
anything. Red-then-green is the anti-Mock-Trap mechanism.

### External

- `cairo`, `pangocairo`, `sysinfo 0.29`, `xcb` — all present; **no new crates required**.
- `scripts/install.sh` — deploy path.
- A live X11 session is required by more than Phase 9 — the earlier claim that "Phases 1–8 verify
  headlessly" was false and is withdrawn. ACs needing a display: Phase 1 AC2/AC3 (live `overlay_cpu`
  and `fps`), Phase 3 AC4 (Z-depth screenshots), Phase 5 AC5 (motion sign-off), Phase 7 AC1 (Pulse CPU),
  Phase 8 AC1 (visible render change), all of Phase 9. Everything else — the MRC, unit tests, config
  fixtures — verifies headlessly.
- **Phases 1–8 use a temporary foreground run of the release binary** for their live ACs:
  `cargo build --release`, then execute `./target/release/matrix-overlay` **directly**. **`cargo run`
  is forbidden for M-1 and S-13b** — it interposes cargo as the parent, so `$!` and any naive pid
  capture yield cargo rather than the overlay, and the sample reads ~0%. That binary must
  not be copied to `~/.local/bin/`, must not touch autostart, and must not terminate the running
  overlay (pid 2462 today). Replacing the deployed binary and restarting the user's overlay happens
  **only in Phase 9**, under C-06 approval. Run the temporary instance alongside or after manually
  stopping the live one at the user's discretion — never by scripted kill.
- **`tests/window_integration.rs` must not run against the user's `$DISPLAY`** (R-11).

## 1.7 Rollback Strategy

| Layer | Mechanism |
|---|---|
| **Per phase** | Each phase is one atomic commit on `refactor/matrixoverlay.v2`. `git revert <sha>` restores the prior state. |
| **Binary** | Phase 9 copies the current binary to `~/.local/bin/matrix-overlay.pre-remediation` **before** installing. Rollback: `cp ~/.local/bin/matrix-overlay.pre-remediation ~/.local/bin/matrix-overlay` and restart. |
| **Config** | Phase 9 backs up `~/.config/matrix-overlay/config.json` to `config.json.pre-remediation`. All new fields are `#[serde(default)]`, so the old config remains loadable by the new binary and vice versa. |
| **Campaign** | Phases 3–4 deliver the CPU win. Phases 6–8 are additive. If Phase 6 destabilizes, revert to the Phase 5 commit and the campaign still meets S-04. |

**Rollback trigger:** any of S-01…S-13 regressing, or user-observed visual regression at the
Phase 3 / Phase 5 sign-off gates.

## 1.8 Verification Method

### The Minimal Reproducible Case (MRC) — mandatory

Established during the 2026-09-03 investigation and reproducible without a display:

```
GIVEN   RainManager::draw at 4096x2160, realism=4, font_size=16,
        streams primed to steady-state distribution
WHEN    40 consecutive frames are rendered through the production code path
THEN    baseline: > 20 ms/frame, flat, no warm-up convergence   (dev profile measured ~750 ms;
                  the --release figure is recorded, not assumed — see "Profile matters" below)
        target:   < 20 ms/frame  (Phase 3, and this gate does not move again)
                  Phase 4 adds a STRUCTURAL gate — zero hot-path show_layout calls — not a
                  tighter millisecond threshold. Any observed 8 ms figure is a receipt metric.

CONTROL The identical glyph count at a single font size. Its cost is RECORDED from its own
        --release run, not asserted at a literal figure — the ~12 ms originally quoted here
        was a dev-profile measurement (round-3 audit).
        This control is what proves the cost is size-churn and not glyph volume —
        and it is precisely the control the existing Mock Trap measured *instead of*
        the real path.
```

This runs as `cargo test --release --test performance_tests`, requires no X server, and **fails today**.

**Profile matters (audit).** `[profile.release]` is `opt-level = "z"`, LTO, `codegen-units = 1`,
`panic = "abort"`. The binary Phase 9 deploys is built by `scripts/install.sh` → `cargo build --release`.
A dev-profile MRC does not describe that binary, so every MRC gate binds to `--release`. Record both
profiles in the receipt; the red-run AC requires only that the mean **exceed the gate**, not that it
print any particular figure.

**What the MRC does not cover.** It times `RainManager::draw` in isolation. It does **not** include the
opaque `clear()`, `rain.update`, the metrics glow passes, `Presenter::pre_draw`'s synchronous
`GetInputFocus` round-trip, `ShmPutImage`, or the second monitor. That gap is S-13, and it is the
reason the Phase 9 halt condition (AC7) exists.

### Verification layers

1. **MRC regression test** — replaces `test_render_optimization_bench`. Gates S-01, S-02, S-08.
2. **Unit tests** — bucket mapping, atlas key/eviction, governor pacing, `overlay_cpu`
   normalization. Gates S-03, S-07.
3. **Config fixture test** — loads the user's real `config.json`. Gates S-11.
4. **Live measurement** — **Method M-1** against the deployed binary. Gates S-04, S-05. *(Round-2
   audit: this layer previously read "`ps`/`top`", the two instruments M-1 exists to replace, and
   listed S-12 — which is a `cmp` of two files, not a CPU reading, and belongs to layer 3.)*
5. **Self-reported telemetry** — the new `fps` metric, cross-checked against wall clock. Gates S-06.
6. **User visual sign-off** — Phase 3 (Z-depth) and Phase 5 (motion smoothness). Gates R-01, R-03, C-05.

### Anti-Mock-Trap discipline (binding on every phase)

> A test that passes against a code path production does not take is worse than no test.
> `test_render_optimization_bench` passed continuously while the code it named was ~90× slower
> than it claimed, because it exercised a single font size through a single layout — the one
> case the renderer never hits.
>
> **Every performance assertion in this campaign must call production code with production-shaped
> inputs.** Where that is impossible, the test is labeled a *control*, never a *validation*.

## 1.9 Falsification Criteria — when F1 is the *wrong* root cause

Every other gate in this plan answers "did the fix work?". None of them answered "was the diagnosis
right?" — A-02 was marked *falsifiable by the Phase 2 MRC*, but nothing said what a falsifying reading
would look like, which meant any disappointing number could be absorbed as "re-derive the arithmetic"
and the campaign would grind on. These criteria close that. **They are evaluated at the mandatory
Phase 2/3 stop, before Phase 3 is permitted to open.**

**F1 (font-cache eviction) is the wrong root cause — halt and re-center — if any ONE of these lands:**

| # | Reading | Threshold | What it means |
|:--:|---|---|---|
| **X-1** | `--release` MRC mean — **only if the MRC is CALIBRATED per Phase 2 AC0** | **≤ 20 ms/frame** | The ~750 ms figure was a dev-profile artifact. Phase 3 has nothing to fix that could explain 61%. **Do not open Phase 3.** Re-center on `present_ms × fps × 2`. |
| **X-2** | `--release` MRC mean vs the single-size control | **within 20% of each other** | A-02 is false *even if both are slow*. The cost is glyph volume or fill rate, not font-cache eviction. Bucketing and the atlas buy the campaign nothing. |
| **X-3** | Phase 1 live `fps` **and** the present budget | `fps ≥ 15` (frames finishing inside the 33 ms tick) **and** `(present_ms_hdmi + present_ms_edp) × fps ÷ 10 ≥ 40` | Present × rate × two CRTCs already accounts for the 61%. F1 is at most a contributor, not the root cause. |

**`fps ≥ 15` with `present_budget_pct < 40` is Branch 1 at the measured rate — X-3 does not fire.**
*(Added round-6, from Phase 1's actual reading: 30.2 fps, 4.85% present budget.)* This cell means the
loop is running at the hardcoded 33 ms tick and present is cheap, so the cost is per-frame Cairo work
multiplied by rate — the bin F1 *would* occupy if it is expensive. **The cell does not classify F1
either way.** What it does establish is that A-01 is falsified and the Phase 5 budget must be
re-derived at the measured rate. Receipt verdict: `F1_STANDS_REDERIVE`. No new falsifier and no new
enum value: X-1 and X-2 remain the only tests of the F1 diagnosis itself.

**`fps ∈ (2, 15)` is Branch 1, not a fourth falsifier.** It means the live rate is not the inferred
1.3; it does not mean font-cache eviction is absent. Re-derive the Phase 5–6 arithmetic, then open
Phase 3 — provided X-1 and X-2 both miss. Re-derivation and falsification are different verdicts and
this band gets the former.

**F1 stands, and the campaign continues into Phase 3, only if all three hold:** the `--release` MRC is
**> 20 ms**, the single-size control is **several times cheaper** than it, and live `fps` is **~1–2**.
Carry the measured numbers forward into the Phase 5 budget identity (§1.3) and proceed.

**X-1 is gated on calibration (round-4 audit).** It is a *fast-green* falsifier, and R-06 only guards
the opposite direction — a synthetic loop producing a slow, falsely-confident number. Nothing guarded
against a subtly wrong MRC (wrong geometry, streams not primed to steady state, sizes not actually
varying) producing a fast green reading that halts a **correct** diagnosis. Worse, Phase 2 AC1 and X-1
draw opposite conclusions from the identical observation: AC1 reads "release ≤ 20 ms ⇒ the test is
wrong, fix the test"; X-1 reads "release ≤ 20 ms ⇒ the diagnosis is wrong, halt Phase 3." The only
thing that separates those two readings is a calibrated slow run on the same test — **Phase 2 AC0**.
X-1 may not be honored until AC0 returns CALIBRATED. An UNCALIBRATED verdict means "this is not the
workload we diagnosed", not "F1 is false": fix the test, do not move the threshold, do not open Phase 3.

### X-LIVE — live-agreement rider on the MRC (added round-6)

AC0 calibrates the MRC against the **investigation** (500–900 ms dev-profile anchor). It cannot catch
an MRC that faithfully measures something *production does not do* — and Phase 1's live data makes
that a live possibility rather than a hypothetical.

**Rule (round-6 form, superseded by the ratio below — kept because it is what tripped).** Once the
`--release` MRC exists: if `mrc.release.mean_ms > 25`, the verdict is **`UNCALIBRATED_VS_LIVE`**. Do
**not** honor X-1. Do **not** open Phase 3. Fix the test — never move the threshold.

### Round-8 disposition — X-LIVE remaining open is a FINDING, not a halt

**`PHASE_2_CLOSED_LAB_DIVERGENT`.** X-LIVE still trips (ratio 60.55) and that is now a recorded
finding — *"the cargo-test MRC is not the live path"* — rather than a campaign halt.

The reasoning is short. The MRC existed to open or block Phase 3. Phase 3 is blocked and demoted on
an **in-process** measurement (1.25× against a 3.00 gate) that needs no MRC at all. Continuing to halt
the campaign on an instrument whose only purpose has been discharged would be a Mock Trap of the
campaign itself — a gate kept green-or-red for its own sake after the decision it fed has been made
by better evidence.

**The test is kept and relabeled `LAB_F1`:** cargo-test reproduces Pango size-churn at 74×; the
overlay process does not, at 1.25×. Both readings are true and they are about different processes.

**Rule (round-7 form, retained for the record).** X-LIVE is a **ratio**, not an absolute:

```
X-LIVE trips when   mrc.release.mean_ms / in_process_rain_draw_4k_ms  >=  3.0
backstop            mrc.release.mean_ms > 25 ms, used only when the in-process figure is absent
```

**Why a ratio.** The absolute 25 ms was derived from a 19.71 ms tick that predates measurement. Once
Phase 3 lands, the live figure moves and 25 ms would have to be re-tuned — and a threshold that has
to be re-tuned after every change is not a falsifier. The ratio needs no re-tuning: it asks the only
question that matters, *does the test measure the same thing the process does*. Both forms trip on
the round-6 reading (612.530 / 10.0030 = 61.2), so nothing is weakened by the change. **Do not
re-tune the 25 ms backstop after Phase 3.**

**Why 25 ms.** The MRC times `RainManager::draw` on **one** surface. The live tick is **19.71 ms
total**, and that budget already pays for present, the second monitor, `clear()`, `rain.update` and
the metrics glow. A 4K draw costing 25 ms or more cannot coexist with a 19.71 ms tick — the two
readings would be describing different programs.

**On the ~750 ms figure.** It is not the same measurement as the live 18.11 ms: dev profile vs
release, `draw`-only vs full tick, one 4K surface vs two CRTCs. But 750 ms per frame inside a process
observed at **30.2 fps** is physically impossible — 750 ms would cap the loop near 1.3 fps, which is
exactly the inference A-01 made. **That number was the 1.3 fps inference talking to itself.** AC0
still tests identity against the investigation; X-LIVE tests identity against the running substrate.
Both must hold before X-1 means anything.

### The AC0-passes / X-LIVE-trips cell (round-7, measured)

| AC0 | X-LIVE | verdict | action |
|---|---|---|---|
| CALIBRATED | does not trip | proceed — X-1 and X-2 may now be honored | normal Phase 2 completion |
| CALIBRATED | **trips** | **`UNCALIBRATED_VS_LIVE`** | **lab F1 is real, the live path disagrees. Halt Phase 3. Rework the MRC — never the threshold.** |
| UNCALIBRATED | either | `UNCALIBRATED` | fix the test; X-1 is not honored |

**This cell is not hypothetical — it is what Phase 2 returned.** The MRC reproduced the investigation
exactly (609.8 ms dev, inside [500,900], 69.2× its control, R-06 satisfied) *and* contradicted the
substrate by 61×. AC0 alone would have certified it and opened Phase 3 on a workload production never
runs. **AC0 is retired as a live gate** and kept only as an investigation-identity record: the
reworked MRC is expected to land near 10 ms and therefore to FAIL [500,900]. That failure is correct.
**Do not re-tune the 500–900 window to chase it.**

**Clearing `UNCALIBRATED_VS_LIVE` requires both:**

1. reworked `mrc.release.mean_ms / in_process_rain_draw_4k_ms < 3.0`, and
2. the surviving-`show_layout` counts on both sides recorded, with their ratio explained.

That clears **Phase 2 completion only**. It does **not** open Phase 3 (see §2.5).

A falsified F1 does **not** invalidate the campaign's instrumentation work. Phases 1 and 2 stand: the
`overlay_cpu` fix, the `fps` metric, the disarmed Mock Trap, S-13a and S-13b are all independently
valuable and independently verified. What changes is which cost centre Phases 3 onward attack.

---

# PART 2 — CAMPAIGN

Structured per the Campaign Planning Framework (Divergence #2). Option F is a multi-day,
multi-front effort with distinct lines of effort and real branch points; the framework fits.

## 2.1 Mission

Restore the Matrix Overlay to its documented performance contract (**<1-3% CPU**, `concept.md` §III)
by eliminating the font-cache eviction defect at the root of a 60× budget overrun — and, in the same
campaign, repair the three instruments whose failure allowed it to persist unseen: a self-report that
under-stated by 16×, a performance test that validated the wrong code path, and a frame rate nobody
could read.

## 2.2 Commander's Intent

**Purpose:** The overlay is ambience. It must cost what ambience costs.

**Method:** Fix the instruments *first*, then the defect, then the architecture — in that order,
because a fix measured by a broken gauge is indistinguishable from a fix that did nothing.

**End State:** The overlay runs under 3% of a core (under 0.5% in Pulse Mode), reports its own cost
accurately, exposes its frame rate directly, and every Performance Preset button does what its label
says. The next agent who opens this repo finds a test that fails when the renderer regresses.

**Freedom of action:** Phases 3 and 4 may be merged if the atlas proves straightforward. Phase 6
may be abandoned wholesale if Phases 3–5 already clear S-04 — the CPU target is the mission, the
architecture is the method.

## 2.3 End State

| Dimension | Now | End State |
|---|---|---|
| CPU (live) | 60.7% of a core | < 3% of a core (< 0.5% Pulse) |
| Rain frame cost | ~~~750 ms (dev profile)~~ **10.0030 ms live, in-process** *(round-7 correction)* | < 20 ms — **already met live**; S-02's zero-hot-path-shaping goal moves to the demoted Phase 3–4 sequel |
| Self-reported CPU | 3.79% (16× low) | Matches Method M-1 ±1pp |
| Frame rate | Unknown / inferred | On-screen metric, ±10% accurate |
| Frame cap | Fails open under load | Holds at configured interval |
| Perf presets | 3 dead buttons, 1 dead field | Drive density, fps, glow, mode |
| Pulse Mode | Promised; draws nothing | Implemented, < 0.5% CPU |
| Perf test | Mock Trap (green while broken) | MRC (red before fix, green after) |

## 2.4 Lines of Effort

**LOE-1 — Instrumentation Truth** *(Phases 1, 2)*
Repair every measuring device before touching the thing being measured. Delivers F2, F6, and the
MRC. Exit: the substrate can no longer lie about its own cost.
→ *Forward contract to LOE-2:* a red MRC and a truthful `overlay_cpu` reading.

**LOE-2 — Root Cause Elimination** *(Phases 3, 4)* — **DEMOTED TO SEQUEL (round-8).** Live `rain.draw`
costs **1.25×** its own in-process single-size control; the re-entry gate is 3.00. Bucketing and the
atlas would attack a cost the live process does not pay. Not dropped, not mission-critical.
Bucketed font sizes with persistent layouts, then a glyph atlas that removes Pango from the frame
path entirely. Delivers F1. Exit: S-01 and S-02 pass; MRC is green.
→ *Forward contract to LOE-3:* per-frame render cost is bounded and known.

**LOE-3 — Temporal Control** *(Phases 5, 6)* — **THE MISSION LEVER (round-8).** `ms_per_tick` is
20.64 measured; at `target_fps = 1` that projects **2.06%** against the 3% S-04 gate, with the rain
path untouched. LOE-2 is not a prerequisite.
Fix the tick thread's fail-open cap, add a real frame governor aligned to the documented refresh
guidance at [docs/pitfalls.md:72](docs/pitfalls.md), then
remove per-frame work that need not happen per frame (damage tracking, mutex-free metric snapshot).
Delivers F4. Exit: S-07 passes; frame cost × frame rate lands inside budget.
→ *Forward contract to LOE-4:* a governed loop with density and rate as tunable inputs.

**LOE-4 — Concept Fulfillment** *(Phases 7, 8)*
Implement the Pulse Mode the concept promises; make the Performance Presets real by wiring them to
the density/rate/glow knobs LOE-3 created. Delivers F5, F7, F8, GL-2. Exit: S-05, S-09 pass.
**Entry is Phase 5, not Phase 6** — Pulse Mode needs a governed interval and a `rain_mode` that
survives startup (F8), nothing from the optional damage-tracking phase.
→ *Forward contract to LOE-5:* a shippable binary meeting all criteria.

**LOE-5 — Deploy & Record** *(Phases 9, 10)*
Rebuild, install, measure live, document. Delivers F3, S-04, S-12, and the pitfalls entry that stops
the next agent re-learning this.

## 2.5 Branches & Sequels

| Trigger | Branch |
|---|---|
| Phase 1's fps metric shows the live rate is **not** ~1.3 fps — including the `fps ∈ (2, 15)` band | A-01 is falsified. **Re-derive the Phase 5–6 frame budget before proceeding**; do not silently continue on stale arithmetic. **F1 does not stand unconditionally here** *(round-4 correction)* — it stands in this band only if §1.9's X-1 and X-2 both miss. A wrong A-01 is a re-derivation, not a falsification; the falsifiers are X-1/X-2/X-3 and nothing else. |
| Phase 3 user sign-off **rejects** the bucketed Z-depth (R-01) | Raise bucket count and re-present. If still rejected, skip to Phase 4 — the atlas supports far more buckets at lower cost, dissolving the trade-off. |
| Phases 3–5 already clear S-04 with margin | **Sequel:** Phase 6 becomes optional. Consult the user before spending a day on architecture the CPU budget no longer requires. |
| Atlas memory exceeds a sane cap (R-02) | Fall back to Phase 3's persistent-layout approach with a reduced bucket count. S-01 still holds; S-02 is waived with the reason recorded. |
| Phase 9 live measurement misses S-04 | **Halt the campaign.** Do not document success. Re-open investigation — a gap between MRC-green and live-red means a second cost centre exists outside the rain path. |
| `perf_preset` proves genuinely unwanted | Phase 8 branch: **remove** the buttons and the field rather than wire them. Ghost Logic is resolved by deletion or by implementation — never by leaving it. |
| The budget identity (§1.3) projects the default preset **above 3%** at Phase 5's chosen `target_fps` | **Lower the default `target_fps` before Phase 9** — do not proceed hoping Phase 6 recovers the difference. Phase 6 is optional by §2.5; the mission is not. `concept.md`'s companion guidance (pitfalls.md:72) already sanctions 1Hz. |
| Phase 7 is reached and F8 (`main.rs:28`) has not been fixed | **Halt Phase 7.** S-05 cannot be measured while startup overwrites `rain_mode`; a "passing" Pulse Mode measured against the fall renderer is Hallucinated Success. |
| Any of §1.9's X-1, X-2 or X-3 lands at the Phase 2/3 stop | **F1 is the wrong root cause. Halt; do not open Phase 3.** Keep Phases 1–2 — instruments, MRC, S-13a/S-13b are all independently valuable — and re-center the campaign on whatever S-13 named. This is a *re-diagnosis*, not a failure of the work already done. |
| **Phase 2's rework leaves X-LIVE tripping but the in-process ratio settles Phase 3** *(round-8 — this fired)* | **`PHASE_2_CLOSED_LAB_DIVERGENT`.** Phase 2's mission deliverables are complete and verified; the MRC is relabeled `LAB_F1` and gates nothing. `phase_2_complete: false` does **not** block downstream work. **Proceed to Phase 5.** |
| **X-LIVE trips while AC0 passes** *(round-7 — this fired)* | **Lab F1 is real; live F1 is not.** MRC 612.5 vs its own control 8.55 = 72× (F1 reproduced in cargo-test); production `rain.draw` 10.0030 vs the same control = 1.17× (production sits *on* the control). Halt Phase 2 for rework, do not open Phase 3, and do not touch the threshold. |
| **Phase 2 rework clears the X-LIVE ratio** | **The next mission phase is Phase 5**, not Phase 3. **Phases 3–4 are DEMOTED to sequels** — Extreme@30 quality work, the same shape Phase 6 already has. They are not dropped and not mission-critical. |
| **Anyone proposes re-opening Phase 3** | Permitted **only** when `live_rain_draw_4k / live_single_size_control_4k >= 3.0`, **both measured in-process inside the running overlay**. The cargo-test control (8.55 ms) is *not* the denominator — it is a different process with a different Pango cache. On today's numbers that ratio is ≈ 1.17 and Phase 3 stays closed. |
| S-13 shows `rest_ms` dominates `rain_ms` after Phase 4 | The rain path is no longer the cost centre. Re-prioritize: the present path, the glow passes, and the `SharedMetrics` lock (Phase 6.1/6.3) become primary rather than optional. Record the reversal explicitly. |

## 2.6 Common Developer Themes — Alignment

Mandatory per Divergence #1. How this plan honors each:

1. **Clarity Over Cleverness** — The atlas is a `HashMap<(char, u8), ImageSurface>`, not a
   hand-rolled cache. Bucketing is integer division on a documented range. Phase 6's damage
   tracking keeps a full-repaint fallback so the clever path is never the only path.
2. **Testability First** — Phase 2 writes the test *before* Phase 3 writes the fix. Every success
   criterion in §1.3 is a command or a reading. The campaign's central artifact is a test that
   fails today.
3. **Minimal Surprise** — New code follows existing shape: collectors implement `MetricCollector`,
   the atlas lives beside `rain_manager.rs` under `render/physics/`, config fields follow the
   `#[serde(default = "default_*")]` idiom already used throughout `defaults.rs`.
4. **Explicit Error Handling** — Atlas construction returns `Result`; a failed glyph render falls
   back to direct `show_layout` rather than panicking. The governor cannot divide by zero
   (`target_fps` clamped ≥ 1). Config values are bounded on read per `concept.md`'s Zero-Trust rule.
5. **Documentation as Code** — Phase 10 writes the font-cache eviction mechanism into
   [pitfalls.md](docs/pitfalls.md) alongside the existing SHM and OverrideRedirect entries. The
   `depth`-must-stay-continuous constraint is an inline comment at the bucketing site, because that
   is where the next person will be tempted to "simplify" it.
6. **Security by Default** — No new inputs, no new network or IPC surface. The atlas is keyed by a
   bounded glyph range (`0x30A1..=0x30F6`) and a bounded bucket index; neither is user-controlled
   free text. Config additions are range-clamped on load.
7. **Performance Awareness** — This *is* the performance plan. Every phase carries a numeric gate.
   The one new allocation (the atlas) is explicitly measured against a cap at Phase 4 (A-04, R-02).
8. **Future-Proofing** — The atlas is keyed by `(char, bucket)`, so a future non-Katakana glyph set
   or a variable font size needs no structural change. The governor exposes `target_fps` as config,
   so the documented "1Hz is sufficient" guidance ([docs/pitfalls.md:72](docs/pitfalls.md)) becomes a
   user setting rather than a recompile.

## 2.7 Failure Patterns Under Active Guard

Named per the global failure vocabulary. Each has a specific countermeasure in this plan:

| Pattern | Instance | Guard |
|---|---|---|
| **Mock Trap** | `test_render_optimization_bench` passed green for months while measuring the one font-size case production never takes | Phase 2 MRC must go **red before** Phase 3 and green after (S-08); R-06 forbids synthetic loops |
| **Ghost Logic** | `perf_preset` field + three GUI buttons wired to nothing; Pulse Mode described in `concept.md`, absent from `pipeline.rs` | Phases 7–8 resolve by implementation; §2.5 permits resolution by deletion — never by leaving them |
| **Hallucinated Success** | Declaring the CPU fixed while measuring with a gauge that reads 16× low | Phase 1 precedes all fixes; S-03 cross-checks against **Method M-1** (§1.3) — never `ps -o pcpu`, which is the lifetime average that would report the *old* cost of a fixed-but-unrestarted process |
| **Sound Effect Execution** | A green MRC on a binary that was never deployed | S-12 `cmp`s the installed file against the just-built `target/release/matrix-overlay` and records the built git sha; Phase 9 measures the *live* process. *(Round-3: this row still described the superseded mtime-vs-HEAD check, which proves only that some file is newer than some commit date.)* |
| **Context Erosion** | A 10-phase campaign drifting from the CPU mission into architecture for its own sake | §2.5 sequel makes Phase 6 abandonable; S-04 is declared the single definition of done |
| **Mock Trap** *(2nd instance)* | `test_stability_no_flicker` asserts `update_ms >= 500` — the metrics collector period — while the render tick is hard-coded 33 ms, so C-05 is "tested" green against a clock production does not use. `test_layout_predictability` has every assertion commented out and can never fail. `render_bench.rs` benches one `FontDescription` and one string | Task 2.5 extended to `asd_tests.rs` and `benches/render_bench.rs`; the flicker test retargets the tick / `target_fps`; the empty test is restored or deleted, never left green |
| **Ghost Logic** *(2nd instance)* | `show_monitor_label` is written by the GUI and read by nothing; `build_logging_enabled` is defaulted and read by nothing; `core/timer.rs` is declared in `core/mod.rs:13` and called by nothing — and is the *only* caller of `factory::create_collectors` | Phase 8 Ghost Logic sweep: wire or delete, no third option. The same rule that governs `perf_preset` governs these |
| **Hallucinated Success** *(measurement instance)* | Declaring S-04 met from a `ps -o pcpu` reading, which is a lifetime average — a fixed binary that has not been restarted still prints the old figure for hours | Method M-1 (§1.3): two `/proc/<pid>/stat` samples over a 300 s window after a known restart |

---

## Execution

Phase-by-phase tasks, acceptance criteria, and receipt destinations: **[tasks.md](tasks.md)**.

Build via `/execute-build`. Receipts are written to `receipts/BUILD_RECEIPTS.md`, relative to the
repository root, created on first write by Phase 1.

*(Audit correction: earlier drafts named `.workflow_state/receipts/BUILD_RECEIPTS.md` twenty-one
times. Neither that file nor the `.workflow_state/` directory exists in this repository. No directory
is created by this planning pass.)*

**Nothing in this plan has been implemented.** This document and `tasks.md` are a plan only.

---

## Audit Absorption — 2026-09-03

An adversarial audit of this plan was performed before any phase was executed. Findings were verified
against the substrate before absorption. Accepted findings are folded into the sections above and into
`tasks.md`. This note records what was accepted and what was rejected, so the next reader can tell the
difference between a gap that was closed and a claim that did not survive.

### Accepted (verified, absorbed)

| Ref | Finding | Verified by | Landed in |
|---|---|---|---|
| F8 | `main.rs:28` overwrites `rain_mode` at startup | `grep -n 'rain_mode = "fall"' src/core/main.rs` → line 28 | §1.1, §1.2, Phase 7.0 |
| F9 | Per-frame cost outside `RainManager::draw` unmeasured | `pipeline.rs:28-38`, `shm.rs:114-146`, `drawing.rs:24-40` (6 `show_layout` per metric), `handlers.rs:48-57` | §1.2, S-13, §1.8 |
| M-1 | `ps -o pcpu` is a lifetime average, not a window; `pgrep -f` over-matches; `ps` and `top` are different instruments | `man ps` (`%CPU` = cputime/lifetime); pid 2462 prints 61.2% over a 25 h lifetime | §1.3 Method M-1 |
| — | Budget identity: S-02 × Phase 5 default fps exceeds S-04 | 8 ms × 10 fps × 1 monitor = 8% of one core | §1.3 identity, §2.5 branch, Phase 5 AC6 |
| — | "Phases 1–8 verify headlessly" contradicted by five ACs | Phase 1 AC2/AC3, 3 AC4, 5 AC5, 7 AC1, 8 AC1 | §1.6 External |
| — | MRC gates bound to dev profile; deployed binary is `opt-level="z"` + LTO | `Cargo.toml [profile.release]`; `scripts/install.sh` | §1.8, S-01/S-08 |
| — | S-06 unimplementable as written: `manager.rs:54` replaces `SharedMetrics.data` wholesale | `sed -n '50,58p' src/metrics/manager.rs` | S-06, §1.2, Phase 1.4-1.5 |
| — | Phase 4 moved the threshold on the test Phase 3 AC5 freezes | Phase 3 AC5 vs Phase 4 AC1 | S-02, Phase 4 AC1 |
| — | Phase 7 wrongly gated behind the abandonable Phase 6 | §1.6 DAG vs §2.5 sequel | §1.6, §2.4, Phase 7 |
| MT-2 | Mock Traps in `asd_tests.rs` and `render_bench.rs` | `asd_tests.rs:42-49` (asserts `update_ms`, tick is 33 ms); `:53-69` (all asserts commented); `render_bench.rs:17-32` | §1.2, §2.7, Phase 2.5 |
| R-11 | `cargo test` maps windows on the live `$DISPLAY`; geometry assert is false on this host | `window_integration.rs:134-149` asserts 1920×1080; `window/mod.rs:48-83` uses RandR → 4096×2160 | §1.5 R-11, Phase 3 AC3 |
| GL-2 | `show_monitor_label`, `build_logging_enabled`, dead `core/timer.rs` | `grep -rn` → written by GUI / defaulted, never read; no `timer::` caller | §1.2, §2.7, Phase 8.7 |
| R-12 | `Presenter::resize` is a no-op; no RandR handler | `shm.rs:150-152`; `handlers.rs:14-41` | §1.5 R-12, Phase 6.4 |
| S-12 | mtime vs HEAD date does not prove the installed bits contain `380107f` | Phase 9 AC3 as written | S-12, Phase 9 AC3 |
| — | Phase 6 panel cache vs the `+0.5 px/frame` scroll | `components.rs:53-57` | Phase 6.2 |
| — | Per-frame `CreateGc`/`FreeGc` in the SHM present path | `shm.rs:114-144` | Phase 6.7 (optional) |
| — | 1Hz/0.5Hz and "no flashing or blinking" misattributed to `concept.md` §IV | `concept.md` §IV states a 500 ms minimum; the quoted text is `docs/pitfalls.md:70-72` | §1.1, §1.4 C-05, Phase 10.7 |

### Rejected (did not survive verification)

| Claim | Why rejected | Verifying command |
|---|---|---|
| "Amend every remaining flat-filename reference to `presentation.rs` in `implementation-plan.md`" | **No such reference existed.** Before this revision, neither planning document mentioned `presentation` in any form; there was nothing to amend. The stale path is only in `CLAUDE.md:97` and `CLAUDE.md:119`, which Phase 10.6 now corrects. *(The name appears in these documents from this revision onward — in this row and in Phase 10.6 — so re-running the command below will now match. Check `git show b15cbc0:implementation-plan.md` to reproduce the original result.)* | At audit time: `grep -n presentation implementation-plan.md tasks.md` → no hits |
| Proposed **R-13**: shared `xcb::Connection` used concurrently by the event thread and the render thread is a data-race risk | **Not a defect.** libxcb is thread-safe by design — a blocking `wait_for_event()` on one thread while another issues requests is XCB's intended usage model and a principal reason it exists rather than Xlib. The `xcb` crate's `Connection` is `Send + Sync`. No R-13 added. | `threads/mod.rs:22-31` (event thread calls only `wait_for_event`) vs `:61-72` (requests) |
| "`manager.rs` calls only `dispatch::init_collectors`, not `factory::create_collectors`" — implying `factory.rs` is dead | **Half right, absorbed differently.** `factory::create_collectors` *is* called — by `src/core/timer.rs:19`. But `core/timer.rs` has no callers itself, so `factory.rs` is reachable only through dead code. Recorded as Ghost Logic (GL-2), which is stronger than the original claim. | `grep -rn create_collectors src/`; `grep -rn 'timer::' src/` → no callers |

### Round 2 — verdict AMEND, absorbed 2026-09-03

The auditor reviewed the absorption and returned **AMEND**: execute Phases 1–2 only, after five
document fixes, then stop and re-gate before Phase 3. All three rejections above were sustained on
review; no finding was reinstated. The five contradictions it found in the absorption itself, all
verified and fixed:

| # | Contradiction the absorption introduced or left | Fix |
|---|---|---|
| 1 | **Phase 1 AC2 was unsatisfiable.** M-1 demands `pgrep -x` return exactly one pid, while the AC deliberately leaves the deployed overlay running *and* starts a second process of the same name — two pids, guard exits 1 by construction | Pin the cargo-run child pid at spawn; exclude the deployed instance by recorded pid. `pgrep -x` stays correct in Phase 9, where only one instance runs |
| 2 | **M-1 was not propagated everywhere the old command lived** — §1.8 layer 4 still read "`ps`/`top`" and wrongly listed S-12 (a `cmp`, not a CPU reading); §2.7 still read "cross-checks against `ps`"; task 9.7 still read "against `ps`" | All three rewritten to M-1; S-12 moved out of the live-measurement layer |
| 3 | **The §IV citation fix stopped at §1.1 and C-05** — Phase 5's header and objective still attributed the 1Hz line to `concept.md` §IV, and task 7.2 still attributed "No flashing or blinking elements" to it | Both retargeted to [docs/pitfalls.md:70-72](docs/pitfalls.md) |
| 4 | **S-02 was restated as a structural gate but two older sentences still asserted 8 ms** — §1.8's MRC target line and §2.3's End State row — re-opening the Phase 4 threshold move that Phase 4 AC1 had just closed | Both rewritten; the 8 ms figure is now explicitly a receipt metric |
| 5 | **Phase 6 still contracted into Phase 7**, contradicting the rewired DAG; its AC2 demanded a resize guarantee unconditionally, contradicting 6.4b which makes resize handling a choice; §1.7's rollback trigger still read S-01…S-12, omitting S-13 | Phase 6 declared a leaf; AC2 made conditional on 6.4b's branch; rollback trigger extended to S-13 |

Smaller: task 1.1 still said "matches `top` semantics" one page after establishing that `top` is not
M-1; and Phase 2 AC2 asserted a literal "~12 ms" for the control test, carrying the same dev-profile
problem AC1 had just shed. Both fixed.

**S-13 was split rather than relocated wholesale.** The proposal was to move it from Phase 9 into
Phase 2. The auditor's counter — accepted — is that `rest_ms` is two different clocks and Phase 2 can
only measure one of them. Phase 2 has a production-geometry Cairo surface but no X connection, no SHM
segment and no RandR, so timing the present path there would be a present-path number taken off a path
production does not take: **a new Mock Trap created in the act of closing the old one.** Hence
**S-13a** (Cairo: `clear`, `rain.update`, glow) in Phase 2.6b, and **S-13b** (X: `GetInputFocus`
round-trip, `ShmPutImage`, `CreateGc`/`FreeGc`, per CRTC) in Phase 1.7, on the live temporary run
Phase 1 already starts. Phase 5 AC6's *"or a recorded estimate otherwise"* escape hatch is deleted —
every term of the budget identity is a measured number before that gate can close.

**Phase 9 AC7's suspect list was replaced with a ranked one**, ordered by *cost the MRC never saw,
paid on both CRTCs*: present × 2 monitors first, then the six-call metrics glow, then the opaque
double `clear()`, then `rain.update` running outside the `"fall"` gate ([pipeline.rs:33](src/render/engine/pipeline.rs#L33)
is not behind the check at line 35 — that is the Pulse-mode leak), and only then the per-present GC
churn and the metrics lock. Three operational suspects — wrong pid, `target_fps` not applied, F8 still
in place — are checked first, because they explain an *absurd* reading rather than a merely high one.

**Execution scope is now gated in `tasks.md`:** Phases 1 and 2 only, then stop. Phase 3 may not open
on the pre-audit arithmetic.

### Round 3 — verdict GO, absorbed 2026-09-03

The auditor returned **GO** on opening Phase 1, with four non-blocking leftovers, one execution-level
correction, and the falsification criteria that were the open question.

**The execution correction was a live defect in the round-2 fix itself.** Phase 1 AC2's pid-pinning
remedy suggested `cargo run --release & TESTPID=$!`. `$!` is **cargo's** pid — cargo spawns the
overlay as a child — so `/proc/$!/stat` would have sampled cargo sitting idle at ~0% and AC2 would
have passed falsely. A Hallucinated Success generator inside the AC written to prevent one. Corrected
to `cargo build --release` followed by running `./target/release/matrix-overlay` directly, with the
log-line and child-walk fallbacks named and `$!` explicitly ruled out.

Four leftovers, all verified and fixed: §2.7's Sound Effect row still described S-12 as an mtime
check after S-12 became a `cmp`; §1.8's CONTROL line still asserted the dev-profile "~12 ms"; §2.4
LOE-3 and §2.6 theme 8 still pinned the 1Hz guidance on `concept.md` §IV; and S-01 / Phase 3 AC1 still
demanded "≥ 40× from ~750 ms", which sets an implicit 2 ms target if the `--release` baseline lands at
80 ms — unsatisfiable for reasons unrelated to the fix. The absolute 20 ms gate is now the binding
one and the ratio is descriptive, computed against the measured release baseline.

**S-13b stays in the Phase 1 build.** The coupling was raised explicitly and judged acceptable: wall
clock around `GetInputFocus` / `ShmPutImage` / `CreateGc` reads neither `overlay_cpu` nor the fps
atomic, so a fault in task 1.1 or 1.4 cannot invent a present-path number, and a timing-only binary
would be a third artifact needing the same hooks and a second live run. One constraint added:
**accumulate internally, print one summary at exit — never log per present**, since a log line on the
path being measured is a new cost centre inside the measurement.

**§1.9 Falsification Criteria is the substantive addition** and closes the gap both prior rounds left.
Every other gate in this plan asked "did the fix work?"; none asked "was the diagnosis right?" A-02
was marked falsifiable by the Phase 2 MRC, but nothing said what a falsifying reading *looked* like —
so any disappointing number could have been absorbed as "re-derive the arithmetic" while the campaign
ground on against the wrong cost centre. Three named readings now say otherwise: **X-1** (a
`--release` MRC at ≤ 20 ms means the 750 ms was a dev-profile artifact), **X-2** (MRC and single-size
control within 20% means A-02 is false even if both are slow — the cost is glyph volume or fill, not
eviction), and **X-3** (live `fps ≥ 15` with present × rate × 2 CRTCs already ≥ 40% means F1 is at
most a contributor). Phase 1 AC6 answers X-3; Phase 2 AC5 answers X-1 and X-2; either landing halts
the campaign at the Phase 2/3 stop. A falsified F1 does not invalidate Phases 1–2 — the instruments,
the disarmed Mock Trap and both S-13 halves stand on their own.

**Status: Phase 1 is authorized to open.** Scope remains Phases 1 and 2, then the mandatory stop.

### Round 4 — final pre-execution pass, absorbed 2026-09-03

Three substantive items, then Phase 1 opens.

**An arithmetic error in the budget identity — present was double-counted.** The single-line form
`(rain_ms + rest_ms) × fps × monitors ÷ 10` folded `present_ms` into `rest_ms` and then multiplied the
whole thing by `monitors`. But `present_ms` is measured **per CRTC** and the two figures are already
summed, so scaling that sum by `monitors` counts every present twice — inflating the projection by
roughly the present cost of one full monitor, in the one gate that decides the default frame rate. The
identity is now two lines: per-surface work (`rain_ms + cairo_rest_ms`) scales by `monitors`;
per-CRTC work (`present_ms_hdmi + present_ms_edp`) is summed and scales by `fps` alone. X-3 was
already correct — it used the second line only — which is what surfaced the inconsistency.

**Phase 2 AC0 — MRC calibration, binding on X-1.** X-1 is a *fast-green* falsifier, and R-06 guards
only the opposite direction. The sharper form of the problem: **AC1 and X-1 draw opposite conclusions
from the identical observation.** A `--release` MRC at ≤ 20 ms means, to AC1, "the test is wrong, fix
the test"; to X-1 it means "the diagnosis is wrong, halt Phase 3." Nothing separated them. AC0 does:
the same test must first reproduce **500–900 ms under the dev profile** and run **≥ 5× the dev
control**, with R-06 confirmed by inspection. Only then is X-1 honored. The 500–900 window is the
2026-09-03 investigation anchor *on this host*, not a universal constant — landing outside it says
"this is not the workload we diagnosed", not "F1 is false". This turns the original investigation
figure from a relic into a calibration instrument.

**The `fps ∈ (2, 15)` band is Branch 1, not a fourth falsifier.** A live rate that is neither ~1.3 nor
≥ 15 means A-01 was wrong; it says nothing about whether font-cache eviction is present. §2.5's
Branch 1 previously implied F1 stood unconditionally whenever A-01 failed — corrected: in that band F1
stands only if X-1 and X-2 both miss. Re-derivation and falsification are different verdicts, and the
receipt's `verdict` field now distinguishes `F1_STANDS_REDERIVE` from `X1`/`X2`/`X3`.

Also: every live-run instruction now reads `cargo build --release` then `./target/release/matrix-overlay`
executed directly. `cargo run` is forbidden for M-1 and S-13b — it interposes cargo as the parent, and
the pid capture yields cargo. Four sites still carried the old wording after the round-3 fix reached
only the AC2 body.

**A receipt schema was added to `tasks.md`** — append-only YAML per phase, with units declared and
every field the budget identity consumes. Phase 5 AC6 runs far enough downstream that nobody present
for this conversation will be reading it; the schema is what makes the identity recomputable from the
receipt alone, and it carries the `verdict` enum that routes the Phase 2/3 stop.

**Status: Phase 1 is authorized to open against this revision.** Scope is Phase 1, then Phase 2, then
the §1.9 evaluation at the stop.

### Corrected in passing

`tasks.md` 1.2 previously described the metric's display label as "Overlay CPU". The actual label at
[metrics/mod.rs:76](src/metrics/mod.rs#L76) is **"HUD CPU"**. The reasoning is unaffected; the string
was wrong and is now accurate.

**No source file, test, `CLAUDE.md`, or `docs/pitfalls.md` was modified by this pass.** Those changes
belong to Phases 1–10. Nothing in the campaign has been implemented.
