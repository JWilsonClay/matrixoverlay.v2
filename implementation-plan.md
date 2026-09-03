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

| `concept.md` promise | Current substrate reality |
|---|---|
| §III "Total system impact target is **<1-3% CPU**" | 60.7% of one core, sustained |
| §II.1 "**Pulse Mode**… static, pulsing glyphs for **<0.5% CPU** impact" | Not implemented. `pipeline.rs:35` draws nothing for any `rain_mode != "fall"` |
| §IV "**1Hz or 0.5Hz is sufficient.** Avoid 60fps animations" | Hard-coded 33ms (30fps) tick, with a cap that fails open under load |

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

Plus Option F architecture: glyph atlas, frame governor, damage-based rendering, `SharedMetrics`
mutex removal from the render path, frame-time histogram, and Pulse Mode implementation.

### Explicitly out of scope

- **The opaque-black `clear()`.** Investigated and found **correct**. [pitfalls.md](docs/pitfalls.md)
  documents `Operator::Source` + `rgba(0,0,0,1.0)` as the CONFIRMED fix from 2026-05-21 for a prior
  regression. The defect is in [CLAUDE.md](CLAUDE.md)'s prose ("Clear… to transparent"), corrected
  in Phase 10 as documentation only. **No code change.**
- Metric collector logic beyond `overlay_cpu` (CPU/RAM/GPU/weather/git collectors untouched).
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
| **S-01** | Rain frame cost collapses | MRC test (§1.8) against production `RainManager::draw` | ≥ 40× faster than baseline; **< 20 ms/frame** at 4096×2160, realism=4 |
| **S-02** | Glyph atlas removes Pango from hot path | MRC, post-Phase 4 | **< 8 ms/frame** same geometry |
| **S-03** | `overlay_cpu` matches `top` | `overlay_cpu` reading vs `ps -o pcpu -p <pid>` | Within **±1 percentage point** |
| **S-04** | Live process meets concept.md §III | `ps aux` on the deployed binary, 5-minute steady state | **< 3% of one core** |
| **S-05** | Pulse Mode meets concept.md §II.1 | Same, with `rain_mode: "pulse"` | **< 0.5% of one core** |
| **S-06** | Frame rate is directly readable | On-screen `fps` metric + `ps` cross-check | Reported fps within **±10%** of a wall-clock count |
| **S-07** | Frame cap holds under load | Governor test: inject a 200 ms frame, observe pacing | Tick never re-queues faster than the configured interval |
| **S-08** | Mock Trap disarmed | `cargo test --test performance_tests` | The replacement MRC **fails before** Phase 3 and **passes after** |
| **S-09** | Presets are real | Click Minimal / Medium / Extreme in the GUI | Config changes on disk **and** render behavior changes |
| **S-10** | Module line limits honored | `wc -l` on every touched file | ≤ **175** lines (concept.md §III) |
| **S-11** | No config regression | Load the user's existing `config.json` unmodified | Parses without error; `deny_unknown_fields` satisfied |
| **S-12** | Deployed binary is current | `stat` mtime vs `git log -1` | Binary newer than HEAD |

**S-04 is the campaign's definition of done.** Everything else is a means to it.

## 1.4 Constraints & Assumptions

### Hard constraints

- **C-01 — Line limits.** 125-line target, **175 hard max** per module (`concept.md` §III).
  `rain_manager.rs` is at 63 today. The atlas requires a **new module**, not growth in place.
- **C-02 — Config compatibility.** `#[serde(deny_unknown_fields)]` is on every config struct.
  Every new field **must** carry `#[serde(default)]` or the user's live config fails to parse
  (CLAUDE.md, Configuration).
- **C-03 — Z-depth fidelity.** Per §1.2 boundary rule.
- **C-04 — X11 / single deploy path.** `scripts/install.sh` is the only supported deploy route.
- **C-05 — ASD design guidance.** `concept.md` §IV: high contrast, **no flashing or blinking**,
  stability preferred. The frame governor must not introduce visible stutter or strobing.
- **C-06 — Live restart requires approval.** Phase 9 restarts the user's running desktop overlay.
  That is a user-visible action and is gated on explicit approval at that phase.

### Assumptions (stated, falsifiable)

- **A-01** — The ~1.3 fps live frame rate is **inferred**, not measured (F6). ptrace_scope=1 blocked
  direct instrumentation and Xvfb is absent. **Phase 1 converts this assumption into a measurement.**
  If Phase 1 shows the live rate is *not* ~1.3 fps, the F1 diagnosis stands (it is independently
  measured) but the frame-budget arithmetic in Phases 5–6 must be re-derived.
- **A-02** — Pango font-cache eviction is the mechanism. Evidence: 0.02 ms for 239
  `set_font_description` calls in isolation, versus ~4.8 ms per size change when a `show_layout`
  follows, versus a drop from 692 ms → 102 ms on the identical workload when live layout references
  are held. Falsifiable by the Phase 2 MRC.
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

## 1.6 Dependencies

### Sequencing (forward contracts — see Part 2 §2.4)

```
Phase 1 (instruments)  ─┬─→ Phase 2 (MRC, red)  ──→ Phase 3 (bucketing, green)  ──→ Phase 4 (atlas)
                        │                                                              │
                        └──────────────────────────────────────────────────────────────┤
                                                                                       ▼
                                              Phase 5 (governor + tick fix)  ──→  Phase 6 (damage + mutex)
                                                                                       │
                                                          Phase 7 (Pulse Mode)  ←──────┤
                                                                                       ▼
                                                          Phase 8 (presets)  ──→  Phase 9 (deploy)  ──→  Phase 10 (docs)
```

**Phase 1 must precede everything.** F2 is the instrument; verifying a CPU fix against a gauge that
reads 16× low is the exact trap that let this defect live for a day.

**Phase 2 must precede Phase 3.** A test written *after* the fix cannot demonstrate it caught
anything. Red-then-green is the anti-Mock-Trap mechanism.

### External

- `cairo`, `pangocairo`, `sysinfo 0.29`, `xcb` — all present; **no new crates required**.
- `scripts/install.sh` — deploy path.
- A live X11 session for Phases 9's verification only. Phases 1–8 verify headlessly.

## 1.7 Rollback Strategy

| Layer | Mechanism |
|---|---|
| **Per phase** | Each phase is one atomic commit on `refactor/matrixoverlay.v2`. `git revert <sha>` restores the prior state. |
| **Binary** | Phase 9 copies the current binary to `~/.local/bin/matrix-overlay.pre-remediation` **before** installing. Rollback: `cp ~/.local/bin/matrix-overlay.pre-remediation ~/.local/bin/matrix-overlay` and restart. |
| **Config** | Phase 9 backs up `~/.config/matrix-overlay/config.json` to `config.json.pre-remediation`. All new fields are `#[serde(default)]`, so the old config remains loadable by the new binary and vice versa. |
| **Campaign** | Phases 3–4 deliver the CPU win. Phases 6–8 are additive. If Phase 6 destabilizes, revert to the Phase 5 commit and the campaign still meets S-04. |

**Rollback trigger:** any of S-01…S-12 regressing, or user-observed visual regression at the
Phase 3 / Phase 5 sign-off gates.

## 1.8 Verification Method

### The Minimal Reproducible Case (MRC) — mandatory

Established during the 2026-09-03 investigation and reproducible without a display:

```
GIVEN   RainManager::draw at 4096x2160, realism=4, font_size=16,
        streams primed to steady-state distribution
WHEN    40 consecutive frames are rendered through the production code path
THEN    baseline: ~750 ms/frame, flat, no warm-up convergence
        target:   < 20 ms/frame  (Phase 3)  →  < 8 ms/frame  (Phase 4)

CONTROL The identical glyph count at a single font size costs ~12 ms.
        This control is what proves the cost is size-churn and not glyph volume —
        and it is precisely the control the existing Mock Trap measured *instead of*
        the real path.
```

This runs as `cargo test`, requires no X server, and **fails today**.

### Verification layers

1. **MRC regression test** — replaces `test_render_optimization_bench`. Gates S-01, S-02, S-08.
2. **Unit tests** — bucket mapping, atlas key/eviction, governor pacing, `overlay_cpu`
   normalization. Gates S-03, S-07.
3. **Config fixture test** — loads the user's real `config.json`. Gates S-11.
4. **Live measurement** — `ps`/`top` against the deployed binary. Gates S-04, S-05, S-12.
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
| Rain frame cost | ~750 ms | < 8 ms |
| Self-reported CPU | 3.79% (16× low) | Matches `top` ±1pp |
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

**LOE-2 — Root Cause Elimination** *(Phases 3, 4)*
Bucketed font sizes with persistent layouts, then a glyph atlas that removes Pango from the frame
path entirely. Delivers F1. Exit: S-01 and S-02 pass; MRC is green.
→ *Forward contract to LOE-3:* per-frame render cost is bounded and known.

**LOE-3 — Temporal Control** *(Phases 5, 6)*
Fix the tick thread's fail-open cap, add a real frame governor aligned to `concept.md` §IV, then
remove per-frame work that need not happen per frame (damage tracking, mutex-free metric snapshot).
Delivers F4. Exit: S-07 passes; frame cost × frame rate lands inside budget.
→ *Forward contract to LOE-4:* a governed loop with density and rate as tunable inputs.

**LOE-4 — Concept Fulfillment** *(Phases 7, 8)*
Implement the Pulse Mode the concept promises; make the Performance Presets real by wiring them to
the density/rate/glow knobs LOE-3 created. Delivers F5, F7. Exit: S-05, S-09 pass.
→ *Forward contract to LOE-5:* a shippable binary meeting all criteria.

**LOE-5 — Deploy & Record** *(Phases 9, 10)*
Rebuild, install, measure live, document. Delivers F3, S-04, S-12, and the pitfalls entry that stops
the next agent re-learning this.

## 2.5 Branches & Sequels

| Trigger | Branch |
|---|---|
| Phase 1's fps metric shows the live rate is **not** ~1.3 fps | A-01 is falsified. F1 stands (independently measured), but re-derive the Phase 5–6 frame budget before proceeding. Do not silently continue on stale arithmetic. |
| Phase 3 user sign-off **rejects** the bucketed Z-depth (R-01) | Raise bucket count and re-present. If still rejected, skip to Phase 4 — the atlas supports far more buckets at lower cost, dissolving the trade-off. |
| Phases 3–5 already clear S-04 with margin | **Sequel:** Phase 6 becomes optional. Consult the user before spending a day on architecture the CPU budget no longer requires. |
| Atlas memory exceeds a sane cap (R-02) | Fall back to Phase 3's persistent-layout approach with a reduced bucket count. S-01 still holds; S-02 is waived with the reason recorded. |
| Phase 9 live measurement misses S-04 | **Halt the campaign.** Do not document success. Re-open investigation — a gap between MRC-green and live-red means a second cost centre exists outside the rain path. |
| `perf_preset` proves genuinely unwanted | Phase 8 branch: **remove** the buttons and the field rather than wire them. Ghost Logic is resolved by deletion or by implementation — never by leaving it. |

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
   so §IV's "1Hz is sufficient" becomes a user setting rather than a recompile.

## 2.7 Failure Patterns Under Active Guard

Named per the global failure vocabulary. Each has a specific countermeasure in this plan:

| Pattern | Instance | Guard |
|---|---|---|
| **Mock Trap** | `test_render_optimization_bench` passed green for months while measuring the one font-size case production never takes | Phase 2 MRC must go **red before** Phase 3 and green after (S-08); R-06 forbids synthetic loops |
| **Ghost Logic** | `perf_preset` field + three GUI buttons wired to nothing; Pulse Mode described in `concept.md`, absent from `pipeline.rs` | Phases 7–8 resolve by implementation; §2.5 permits resolution by deletion — never by leaving them |
| **Hallucinated Success** | Declaring the CPU fixed while measuring with a gauge that reads 16× low | Phase 1 precedes all fixes; S-03 cross-checks against `ps` |
| **Sound Effect Execution** | A green MRC on a binary that was never deployed | S-12 checks binary mtime against HEAD; Phase 9 measures the *live* process |
| **Context Erosion** | A 10-phase campaign drifting from the CPU mission into architecture for its own sake | §2.5 sequel makes Phase 6 abandonable; S-04 is declared the single definition of done |

---

## Execution

Phase-by-phase tasks, acceptance criteria, and receipt destinations: **[tasks.md](tasks.md)**.

Build via `/execute-build`. Receipts are written to:
`/home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md`

**Nothing in this plan has been implemented.** This document and `tasks.md` are a plan only.
