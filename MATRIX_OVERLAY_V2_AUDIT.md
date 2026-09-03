# Audit: Matrix Overlay v2 — Render Substrate Remediation Campaign

**Subject:** `implementation-plan.md` + `tasks.md` on `refactor/matrixoverlay.v2` @ `b15cbc0`
**Date:** 2026-09-03
**Scope:** Plan revision prompt only. No source edits.
**Excluded from the ledger (already accepted, absorb in the prompt only):** `src/core/main.rs:28` rain_mode clobber; missing source-investigation file; missing `.workflow_state/receipts/BUILD_RECEIPTS.md`; stale `presentation.rs` path.

---

## Section 1 — Findings Ledger

| # | Severity | Type | Failure pattern | Finding, in one sentence | Evidence | Affected phase(s) | Proposed remedy, in one sentence |
|---|---|---|---|---|---|---|---|
| 1 | Critical | Miss | Hallucinated Success | S-03/S-04/S-05 are specified as a 5-minute steady-state reading but the named command is Linux `ps` `%CPU`, which is `100 * cputime / lifetime`, so a process that ran 23.5 h at 61% still prints ~60% after a successful fix until it is restarted, and `pgrep -f matrix-overlay` is unquoted and matches cargo/repo paths. | `implementation-plan.md` §1.3 S-03/S-04; `tasks.md` Phase 1 AC2 and Phase 9 AC1: `ps -o pcpu= -p $(pgrep -f matrix-overlay)`; S-03 title says “matches `top`” while the measurement column is `ps`. | 1, 7, 9 | Replace every live-CPU AC with one named method: pin `pgrep -x matrix-overlay`, take two `/proc/<pid>/stat` samples (or `pidstat`) over 300 s *after a known restart*, and stop citing `top` and `ps` as if they were the same instrument. |
| 2 | Critical | Miss | Context Erosion | The written gates allow S-02 (`< 8 ms` rain draw) × Phase 5 default `target_fps: 10` × Phase 8 Medium preset (the S-04 configuration) = 80 ms of CPU per second ≈ 8% of one core *for 4K rain draw alone*, before the second monitor or SHM present, so S-01–S-02–S-08 can all pass and S-04 still fail. | `implementation-plan.md` §1.3 S-02 / S-04; `tasks.md` Phase 5.3 default **10**; Phase 8.2 Medium = `target_fps` 10, `rain_mode` fall; `src/core/config/defaults.rs` `default_preset() -> "medium"`. | 5, 8, 9 | Publish the budget identity `frame_ms × fps × monitors + present_ms` in §1.3, set the S-04 preset’s default `target_fps` to a value that can clear 3% given the S-02 ceiling (1–2, or prove atlas << 8 ms), and mark Extreme 30 fps as an opt-in that is allowed to miss S-04. |
| 3 | Critical | Miss | Hallucinated Success | Phase 1 AC2/AC3 demand a running binary that already contains the Phase 1 instrument fixes, but C-06 / Phase 9 is the only approved live restart, PID 2462 cannot certify S-03, and §1.6 simultaneously claims Phases 1–8 are headless. | `tasks.md` Phase 1 AC2/AC3; Phase 9 + C-06; `implementation-plan.md` §1.6 External: “Phases 1–8 verify headlessly. A live X11 session for Phase 9 only.” Also contradicted by Phase 3 AC4 screenshots, Phase 5 AC5 sign-off, Phase 7 AC1, Phase 8 AC1. | 1, 3, 5, 7, 8, 9 | Split instruments: Phase 1 live ACs run against a temporary `cargo run --release` that does not touch autostart; S-03/S-04/S-05 on `~/.local/bin/matrix-overlay` move to Phase 9; amend §1.6 so it lists which ACs need a display. |
| 4 | High | Miss | — | S-01/S-02/S-08 time only `RainManager::draw`; production also does opaque full-surface clear, `rain.update` unconditionally, metrics + glow, `GetInputFocus` + `ShmPutImage` per monitor, under a mutex, on *two* CRTCs (4096×2160 + 1920×1080). | `pipeline.rs:28-38`; `drawing.rs:24-38`; `presentation/shm.rs:101-147`; `handlers.rs:51-57`; `window/mod.rs:48-80` RandR loop. | 2, 6, 9 | Add **S-13**: a present-path / dual-monitor budget (or an explicit named suspect list for the existing Phase 9 “MRC-green / live-red” halt), and extend the MRC or add a control that calls `Renderer::draw` / `present()` rather than rain blit alone. |
| 5 | High | Miss | Hallucinated Success | S-01/S-02/S-08 are `cargo test` times (dev profile); the S-04 binary is `scripts/install.sh` → `cargo build --release` with `opt-level = "z"`, LTO, strip; Phase 2 AC1’s “FAILS reporting ~750 ms” is a debug-profile number and may be false under `--release`. | `implementation-plan.md` §1.8; `tasks.md` Phase 2 AC1; `Cargo.toml` `[profile.release]`; `scripts/install.sh`. | 2, 3, 4, 9 | Record MRC under both profiles; bind S-01/S-02/S-08 gates to `cargo test --release --test performance_tests` so they describe the binary Phase 9 deploys. |
| 6 | High | Miss | Mock Trap | S-06 cannot be implemented as written: `manager.rs` replaces `SharedMetrics.data` from collectors every `update_ms` (a renderer-published `fps` is clobbered), `pipeline.rs:29` increments `frames` on both tick and Expose so the counter is not a present count, and §1.3’s “`ps` cross-check” cannot measure fps. | `src/metrics/manager.rs:34` + collector write path; `pipeline.rs:29`; `handlers.rs:31-36` Expose and `44-57` tick; `implementation-plan.md` §1.3 S-06 Measurement column; `tasks.md` 1.4–1.5. | 1, 6 | Source fps from an `AtomicU64` incremented in `Presenter::present` (not `frames++` in `draw`), have a collector read it so `manager.rs` does not drop it, and replace the `ps` cross-check with an independent 10 s wall-clock present count. |
| 7 | High | Miss | Hallucinated Success | Phase 4 AC1 tightens the same MRC threshold Phase 3 just turned green; if bucketing alone yields `< 8 ms`, Phase 4 is already green before any atlas work. | `tasks.md` Phase 3 AC1 / AC5 (`git diff --stat tests/` must be unmodified); Phase 4 AC1 “threshold tightened from 20 ms”. | 3, 4 | Keep the 20 ms test file untouched in Phase 4; add a *new* S-02 assertion (hot-path `show_layout` count == 0, or atlas hit rate) instead of moving the ms gate. |
| 8 | High | Miss | Context Erosion | §1.6 draws Phase 7 ← Phase 6, but Pulse Mode needs a governed interval and a `rain_mode` that reaches the pipeline, not damage tracking; parking S-05 behind the abandonable phase delays the concept contract. | `implementation-plan.md` §1.6 DAG; §2.5 sequel already makes Phase 6 optional; `tasks.md` Phase 7 forward-contract prose. | 6, 7 | Rewire the DAG: Phase 7 depends on Phase 5 (+ the already-accepted `main.rs:28` gate); Phase 6 stays parallel and skippable. |
| 9 | High | Miss | Mock Trap | `test_stability_no_flicker` asserts `general.update_ms >= 500` (metrics collector period) while the overlay tick is hard-coded 33 ms, so C-05 is “tested” green against a clock production does not use. | `tests/asd_tests.rs:42-49`; `src/core/threads/mod.rs:114-116`; `concept.md` §IV; `tasks.md` 2.5 names only `performance_tests.rs`. | 2, 5 | Extend task 2.5 to `asd_tests.rs` and retarget the flicker test at the tick / future `target_fps`, not `update_ms`. |
| 10 | High | Miss | — | `cargo test` on the daily-driver DISPLAY maps real `_NET_WM_WINDOW_TYPE_DESKTOP` windows, and `test_geometry_and_visual` asserts 1920×1080 at (0,0) against a comment that is false — `create_all_windows` uses RandR, so on HDMI-1-0 the test hard-fails and Phase 3 AC3 (`cargo test` fully green) cannot pass on this host. | `tests/window_integration.rs:127-149`; `src/core/window/mod.rs:55-80`; `tasks.md` Phase 3 AC3. | 2, 3, 9 | Add **R-11**: campaign `cargo test` must not call `create_all_windows` on `$DISPLAY` without Xephyr/Xvfb; fix or `#[ignore]` the 1920×1080 assert; do not let Phase 3 AC3 depend on a known-false geometry test. |
| 11 | Medium | Miss | Ghost Logic | `show_monitor_label` and `logging.build_logging_enabled` are declared, defaulted, and (for the former) written by the GUI, and neither is read by layout, render, or the logger. | `types.rs:20`, `types.rs:130`; `ui/gui/general.rs:59` + `logic.rs:21`; `rg show_monitor_label` / `rg build_logging_enabled` over `src/` — no consumer outside types/defaults/GUI. | 8, 10 | In Phase 8/10 Ghost Logic sweep: wire each field or delete the widget/field; do not leave a checkbox that saves a bit nothing reads. |
| 12 | Medium | Miss | Sound Effect Execution | Task 1.5 writes `fps` into `ui/gui/metrics.rs` and `factory.rs`, but `ConfigWindow::show` never `append_page`s the Metrics tab (`metrics::build` / `productivity::build` are unreachable) and `manager.rs` calls only `dispatch::init_collectors`, so those two edits do not reach production. | `src/ui/gui/mod.rs:38-53`; `src/ui/gui/metrics.rs:9`; `src/metrics/manager.rs:34,45`; `src/metrics/factory.rs:11`. | 1 | Restrict Phase 1.5 to `MetricId` + `dispatch.rs` (+ the atomic/collector in finding 6); treat Metrics-tab wiring as a separate optional task or drop it. |
| 13 | Medium | Miss | Mock Trap | Task 2.5 audits only `performance_tests.rs`; `benches/render_bench.rs` is the same single-`FontDescription` synthetic glow path, and `test_layout_predictability` has every assert commented out and is permanently green. | `benches/render_bench.rs:17-32`; `tests/asd_tests.rs:53-69`. | 2, 10 | Extend 2.5 to the Criterion bench and `asd_tests.rs`; label the bench a control or point it at `RainManager::draw`; restore or delete the empty predictability test. |
| 14 | Medium | Miss | — | `Presenter::resize` is a no-op on both SHM and socket paths, and the overlay thread does not handle `RRNotify`, so Phase 6.4 `force_full_redraw` on resize cannot realloc the 4K SHM segment. | `presentation/shm.rs:150-152`; `presentation/socket.rs` `fn resize`; `handlers.rs:19-41` matches KeyPress + Expose only. | 6 | Add **R-12**: either implement `resize()` as a real SHM rebuild in Phase 6, or explicitly defer RandR and drop “resize” from 6.4’s event list. |
| 15 | Medium | Miss | Sound Effect Execution | S-12 (`stat` mtime vs `git log -1 --format=%ci`) proves the installed file is newer than some HEAD commit date, not that commit `380107f` is in the text of `~/.local/bin/matrix-overlay`. | `tasks.md` Phase 9 AC3; `implementation-plan.md` §1.3 S-12. | 9 | Compare the installed binary to `target/release/matrix-overlay` after `install.sh`, and embed / `strings` a git sha; keep mtime as a secondary check only. |
| 16 | Medium | Miss | — | Phase 6 metrics-panel cache will either freeze the `+0.5 px/frame` scroll in `components.rs` or keep the panel dirty every frame, cancelling the glow-pass saving 6.5 claims. | `src/render/layout/components.rs:53-57`; `tasks.md` 6.2–6.5. | 6 | In 6.2, treat an in-flight scroll offset as dirty, or disable scroll when the panel cache is on (C-05 prefers static text). |
| 17 | Low | Opportunity | — | `ShmPresenter::present` `CreateGc` + `FreeGc` every frame on every monitor is cheap to lift once that file is open. | `presentation/shm.rs:119-144`. | 6 | Persist one `Gcontext` on `ShmPresenter` in Phase 6.1’s present-path pass; skip if Phase 6 is abandoned. |

### Unverifiable / needs machine access

- Exact present-path milliseconds on HDMI-1-0 + eDP (command: instrument `Presenter::present` + `pre_draw` with `Instant` for 40 frames on the live session, or `perf stat` on pid 2462). Settles how large S-13’s budget must be.
- Whether `libxcb` concurrent use of one `Connection` from the wait-for-event thread and the present thread has faulted on this host (command: `dmesg`/`coredumpctl` + a review of `threads/mod.rs:22-31` vs `61-72`). If confirmed, add **R-13**; do not add it to the plan without that check.
- Whether `cargo test --release --test performance_tests` already comes in under 20 ms today on this CPU (command: run it). Settles finding 5’s magnitude.

---

## Section 2 — The Prompt for Claude Code

Copy everything inside the following fence into Claude Code.

```
You are revising two documents only:
  - implementation-plan.md
  - tasks.md

Branch: refactor/matrixoverlay.v2
Campaign: Render Substrate Remediation, Option F, plan id 20260903-matrixoverlay-render-remediation
Nothing in the campaign has been implemented. You are not to implement Rust, tests, or deploy. You are to absorb an audit into the plan and the task list.

================================================================================
1. OBJECTIVE
================================================================================

Revise implementation-plan.md and tasks.md so the campaign can actually hit S-04
(live process < 3% of one core) without inventing work and without restarting
the campaign. Absorb the findings below after verifying each one against the
tree. Do not change the mission. Do not rewrite the [INTENT] User Objective
block. Inject and append. Delete only a sentence that directly contradicts a
correction you are adding, and record the contradiction in the same edit.

================================================================================
2. PRESERVATION RULES (BINDING)
================================================================================

- The block headed "## [INTENT] User Objective" in implementation-plan.md is
  /nodelete. Do not rewrite, condense, relocate, or remove it. You may not
  even append inside that block.
- Do not renumber existing F-nn, S-nn, R-nn, C-nn, A-nn, phase numbers, or
  task numbers. New items continue the series (F8…, S-13…, R-11…).
- Do not delete an existing success criterion, risk, task, or AC unless it
  is factually wrong as written AND you replace it in place with a corrected
  measurement that still gates the same claim. Prefer amendment over deletion.
- Do not implement code, do not edit CLAUDE.md / pitfalls.md / source, do not
  create .workflow_state/. Those are later phases.
- Line limits C-01, config compatibility C-02, Z-depth C-03, ASD C-05, X11-only
  remain in force on anything you add.

================================================================================
3. ALREADY-ACCEPTED AMENDMENTS (DO NOT RE-INVESTIGATE; DO ABSORB)
================================================================================

These four items were accepted before this audit. Fold them in. Do not open a
new investigation of them.

A. src/core/main.rs:28 unconditionally assigns
   config.cosmetics.rain_mode = "fall".to_string();
   immediately after Config::load(), discarding the user's configured mode.
   This blocks Phase 7 S-05 and makes Phase 8 Minimal (rain_mode: pulse) a
   write the next launch overwrites. Add a Phase 7 task BEFORE 7.1 that
   removes or gates that assignment, with an AC: configured rain_mode
   survives process start (read the in-process Config after startup, not
   only the JSON on disk). Name the defect F8.

B. The F1–F7 investigation report is not a file. A-02's 0.02 / 4.8 /
   692→102 ms numbers live only in a transcript. Add a Phase 2 task that
   records the MRC's first red run (profile, mean ms, machine) into the
   receipt so A-02 becomes auditable going forward. Do not invent a
   backdated investigation file.

C. tasks.md names
   /home/jwils/matrixoverlay.v2/.workflow_state/receipts/BUILD_RECEIPTS.md
   twenty-one times. The path does not exist. Replace the receipt line with
   a path that will exist when Phase 1 starts (create-on-write is fine) OR
   state "receipts/BUILD_RECEIPTS.md at repo root, created by Phase 1".
   Do not require a .workflow_state directory that the repo does not have.

D. src/render/engine/presentation.rs does not exist. The Presenter lives at
   src/render/engine/presentation/{mod,shm,socket}.rs (commit e948079).
   Amend every remaining flat-filename reference in implementation-plan.md.
   Add one bullet under Phase 10.2: also correct CLAUDE.md:97 and :119 from
   presentation.rs to the directory. Do not edit CLAUDE.md yourself.

================================================================================
4. FINDINGS TO ABSORB (VERIFY EACH AGAINST THE TREE FIRST)
================================================================================

You MUST open the cited file and confirm the line before writing the finding
into the plan. If a citation does not survive, OMIT that finding and report
it under "Rejected findings" at the end of your working notes. Do not
silently keep a finding you could not confirm. You are encouraged to reject
with evidence.

F-A. LIVE CPU COMMANDS DO NOT MEASURE A 5-MINUTE WINDOW
     Evidence: implementation-plan.md §1.3 S-03/S-04; tasks.md Phase 1 AC2,
     Phase 9 AC1 (`ps -o pcpu= -p $(pgrep -f matrix-overlay)`).
     Linux ps %CPU is 100*cputime/lifetime. pgrep -f is unquoted and matches
     cargo/repo strings. S-03's title says "matches top"; the measurement is
     ps. These are different instruments.
     Remedy: rewrite S-03, S-04, S-05, Phase 1 AC2, Phase 7 AC1, Phase 9 AC1
     to one method: PID=$(pgrep -x matrix-overlay) exactly one pid; two
     /proc/$PID/stat samples (utime+stime) 300 s apart after a known
     restart; report 100*(d_ticks/HZ)/300. Stop saying "ps aux" or "top"
     as if they were that method.

F-B. S-04 ARITHMETIC VS DEFAULT 10 FPS
     Evidence: S-02 < 8 ms/frame; tasks.md Phase 5.3 default target_fps 10;
     Phase 8.2 Medium = 10 fps + fall; defaults.rs default_preset() = "medium";
     Phase 9.5 measures "the default preset".
     8 ms × 10 fps = 8% of one core for 4K rain draw alone, before eDP and
     before SHM present. The written gates can all pass and S-04 still fail.
     Remedy: add the identity
       cpu_pct ≈ (rain_ms + present_ms) * fps * monitors / 10
     to §1.3. Change the S-04 configuration (Medium / default target_fps)
     to a rate that can clear 3% given the S-02 ceiling, OR add a Branch
     that Phase 5 may not close until a measured (rain_ms+present_ms)*fps
     projects under 3%. Extreme 30 fps is opt-in and is allowed to miss
     S-04; say so in Phase 8.2.

F-C. PHASE 1 LIVE ACS VS C-06 VS "HEADLESS 1–8"
     Evidence: tasks.md Phase 1 AC2/AC3; Phase 9 + C-06; implementation-plan.md
     §1.6 External sentence "Phases 1–8 verify headlessly."
     Also contradicted by Phase 3 AC4, Phase 5 AC5, Phase 7 AC1, Phase 8 AC1.
     Remedy: amend §1.6 External. Phase 1 live ACs use a temporary
     `cargo run --release` that does not replace ~/.local/bin/matrix-overlay
     and does not restart pid 2462. S-03/S-04/S-05 against the autostart
     binary live only in Phase 9.

F-D. UNMEASURED PRESENT × DUAL MONITOR (new S-13)
     Evidence: pipeline.rs:28-38; drawing.rs:24-38 (glow_passes loop PLUS a
     sixth full-alpha show_layout); presentation/shm.rs:101-147
     (GetInputFocus + CreateGc + PutImage + FreeGc per frame);
     handlers.rs:51-57 lock held across every monitor; window/mod.rs:55-80
     RandR discovers HDMI-1-0 4096×2160 and eDP 1920×1080.
     S-01/S-02 time only RainManager::draw.
     Remedy: add S-13 — either an MRC control that calls Renderer::draw /
     present() at production geometry, or a documented suspect list the
     Phase 9 halt condition already names (present_ms, second monitor,
     GetInputFocus, glow). Put the dual-monitor multiplier in §1.3.
     New finding id: F9 "per-frame cost outside RainManager::draw".

F-E. DEBUG MRC VS RELEASE BINARY
     Evidence: Cargo.toml [profile.release] opt-level=z lto strip;
     scripts/install.sh cargo build --release; Phase 2 AC1 "~750 ms".
     Remedy: S-01/S-02/S-08 and Phase 2/3/4 MRC ACs run
     `cargo test --release --test performance_tests`. Record both profiles
     in the receipt. Drop the requirement that the red run print "~750 ms";
     require only that it exceed the gate (fail-as-red).

F-F. S-06 IS NOT IMPLEMENTABLE AS WRITTEN (new measurement, not new mission)
     Evidence: metrics/manager.rs rebuilds SharedMetrics.data from collectors
     every update_ms; pipeline.rs:29 increments frames on every draw;
     handlers.rs Expose path 31-36 also calls draw; §1.3 S-06 says "ps
     cross-check".
     Remedy: amend tasks 1.4–1.5. Increment an AtomicU64 inside
     Presenter::present (one count per successful present, not per draw).
     A small FpsCollector reads that atomic. manager.rs therefore cannot
     clobber it. Cross-check is a 10 s wall-clock count of present calls
     (log line or test hook), never ps. Mention manager.rs in Phase 1
     even though "collector logic beyond overlay_cpu" is otherwise out of
     scope — S-06 cannot exist without that touch.

F-G. PHASE 4 MOVES THE THRESHOLD PHASE 3 JUST GREENED
     Evidence: tasks.md Phase 3 AC5 forbids modifying tests/; Phase 4 AC1
     tightens the same test to < 8 ms.
     Remedy: Phase 4 does not edit test_rain_frame_cost_mrc's 20 ms gate.
     Add a new test or AC for S-02: hot-path pangocairo show_layout count
     is zero on the rain glyphs (atlas blit only), or an atlas hit-rate
     assertion. The 8 ms number may remain as an observed receipt metric,
     not as a moved threshold on the Phase 3 test.

F-H. PHASE 7 MUST NOT DEPEND ON PHASE 6
     Evidence: implementation-plan.md §1.6 DAG; §2.5 sequel already makes
     Phase 6 optional.
     Remedy: redraw the DAG so Phase 7 depends on Phase 5 + F8 (main.rs:28).
     Phase 6 remains abandonable and parallel. Pulse does not need damage
     tracking.

F-I. ASD FLICKER TEST MEASURES THE WRONG CLOCK
     Evidence: tests/asd_tests.rs:42-49 assert update_ms >= 500;
     threads/mod.rs:114-116 interval = 33 ms.
     Remedy: extend task 2.5 to asd_tests.rs. The flicker test must assert
     the overlay tick (or future target_fps), not the metrics collector
     interval. Do not implement the test now; specify the change.

F-J. CARGO TEST MUTATES THE LIVE DESKTOP (new R-11)
     Evidence: tests/window_integration.rs:27-35, 97-105, 127-149 call
     create_all_windows on $DISPLAY; L143-149 assert 1920×1080 at (0,0)
     with a comment claiming create_all_windows hardcodes that size;
     window/mod.rs:55-80 uses RandR (production geometry is 4096×2160 +
     1920×1080). Phase 3 AC3 is `cargo test` fully green.
     Remedy: add R-11. Amend Phase 3 AC3 so it does not require
     window_integration to pass on the daily-driver DISPLAY. Instruct
     Phase 2/10 to ignore or fix the 1920×1080 assert (docs only: write
     the task; do not edit the test file in this pass).

F-K. GHOST LOGIC BEYOND perf_preset
     Evidence: general.show_monitor_label is written in logic.rs:21 /
     general.rs:59 and never read by layout or render.
     logging.build_logging_enabled is defaulted in types.rs:130-141 and
     never read.
     ui/gui/mod.rs:38-53 never appends the Metrics or Productivity tabs,
     so tasks.md 1.5's edit to ui/gui/metrics.rs:16 is Sound Effect
     Execution. manager.rs:34,45 calls dispatch::init_collectors only,
     not factory.rs::create_collectors.
     Remedy: add a Ghost Logic bullet under Phase 8 or 10 naming those
     two fields (wire or delete). Rewrite task 1.5 to register fps in
     MetricId + dispatch.rs only. Metrics-tab wiring is out of scope
     unless you add a clearly-optional Phase 8/10 task.

F-L. MOCK TRAPS OUTSIDE performance_tests.rs
     Evidence: benches/render_bench.rs:17-32 single FontDescription, one
     string, not RainManager::draw. tests/asd_tests.rs:53-69
     test_layout_predictability — every assert commented out.
     Remedy: extend task 2.5 to both files. Bench = control, or pointed
     at production draw. Empty predictability test = restore asserts or
     delete the test (specify; do not edit the file now).

F-M. RESIZE IS A NO-OP (new R-12)
     Evidence: presentation/shm.rs:150-152 resize returns Ok(());
     handlers.rs:19-41 has no RRNotify / ConfigureNotify resize path.
     Phase 6.4 lists resize as a force_full_redraw trigger.
     Remedy: add R-12. Either Phase 6 grows a real SHM rebuild (new
     module if shm.rs would exceed 175 lines) or Phase 6.4 drops
     "resize" from the event list and defers RandR explicitly.

F-N. S-12 DOES NOT PROVE 380107f IS IN THE INSTALLED BITS
     Evidence: tasks.md Phase 9 AC3 stat vs git log -1 --format=%ci.
     Remedy: amend S-12 / Phase 9 AC3: after scripts/install.sh,
     cmp ~/.local/bin/matrix-overlay target/release/matrix-overlay
     (or equivalent) and record the built git sha. mtime stays secondary.

F-O. PHASE 6 PANEL CACHE VS SCROLL
     Evidence: components.rs:53-57 advances a scroll offset by 0.5 px
     every draw; tasks.md 6.2-6.5 cache the metrics panel on value change.
     Remedy: add one sentence to 6.2: in-flight scroll counts as dirty,
     or scroll is disabled when the cache is active (C-05 prefers static).

F-P. OPTIONAL, PHASE 6 ONLY
     Evidence: presentation/shm.rs:119-144 CreateGc+FreeGc every present.
     If and only if Phase 6 opens that file, persist one Gcontext on
     ShmPresenter. If Phase 6 is abandoned, drop this.

Do not add findings about OverrideRedirect, Operator::Clear, the SHM
pre_draw race, perf_preset being unread, preset buttons being unwired,
or test_render_optimization_bench — those are already in the plan.

================================================================================
5. PER-DOCUMENT INSTRUCTIONS
================================================================================

implementation-plan.md
  - Do not touch "## [INTENT] User Objective".
  - §1.1 table: the §IV "1Hz" quote is not in concept.md §IV (that section
    is 500 ms minimum interval). The sentence lives at docs/pitfalls.md:72.
    Correct the citation. Do not change the policy.
  - §1.2 In scope: append F8 (main.rs:28 clobber) and F9 (per-frame cost
    outside RainManager::draw). Do not renumber F1–F7.
  - §1.3: amend S-03, S-04, S-05, S-06, S-12 measurement/gate text per
    F-A, F-B, F-F, F-N. Append S-13 for the present/dual-monitor budget.
    Add the cpu_pct identity under the table.
  - §1.5: append R-11 (cargo test maps DESKTOP windows on $DISPLAY),
    R-12 (resize no-op). Only append R-13 (shared xcb::Connection across
    the wait-for-event thread and the present thread, threads/mod.rs)
    if you confirm the concurrent-use fact in the xcb crate docs or
    source; otherwise leave R-13 out and note it as unverified.
  - §1.6: redraw the DAG so Phase 7 ← Phase 5, not Phase 6. Amend the
    External bullet that claims Phases 1–8 are headless.
  - §1.8: MRC command uses cargo test --release. State that the MRC does
    not include SHM present or the second monitor, and that S-13 / Phase 9
    halt exist because of that.
  - §2.4 / §2.5: Phase 7 no longer waits on Phase 6. Add a branch: if
    (rain_ms+present_ms)*default_fps projects above 3%, lower default
    target_fps before Phase 9 rather than hoping Phase 6 saves it.
  - Execution footer: fix the BUILD_RECEIPTS path (already-accepted C).

tasks.md
  - Replace every BUILD_RECEIPTS absolute path with the new path chosen
    above (already-accepted C).
  - Phase 1: amend AC2/AC3 per F-A and F-C. Rewrite 1.4–1.5 per F-F and
    F-K. AC1 `cargo test` must not be read as "including
    window_integration on $DISPLAY".
  - Phase 2: extend 2.5 to asd_tests.rs and benches/render_bench.rs
    (F-I, F-L). Amend AC1 so a red run is "mean > 20 ms", not
    "reporting ~750 ms". Specify --release.
  - Phase 3: amend AC3 so window_integration geometry is not a gate
    (F-J). Keep AC5 (do not modify the Phase 2 MRC file).
  - Phase 4: amend AC1 per F-G (new atlas assertion, 20 ms test stays).
  - Phase 5: add an AC that (rain_ms + present_ms) * target_fps projects
    under S-04 at the default preset, using S-13's present number if
    measured, else a recorded estimate. Revisit default target_fps
    per F-B.
  - Phase 6: add R-12 handling (F-M), scroll/dirty rule (F-O), optional
    persistent GC (F-P). Keep abandonable.
  - Phase 7: INSERT a new first task (7.0 or unnumbered "Prerequisite")
    that removes/gates main.rs:28, with the restart-survival AC
    (already-accepted A). Do not renumber 7.1–7.6 if you can avoid it;
    a "7.0" prefix is acceptable. State dependency on Phase 5, not 6.
  - Phase 8: Medium/default must be S-04-capable (F-B). Extreme may miss
    S-04. Add the Ghost Logic sweep for show_monitor_label and
    build_logging_enabled (F-K) as a task — wire or delete, no third
    option.
  - Phase 9: rewrite AC1/AC2/AC3 per F-A and F-N. Name S-13 suspects in
    AC7's halt text.
  - Phase 10: add the presentation/ directory filename fix next to 10.2
    (already-accepted D). Add the concept.md §IV vs pitfalls.md:72
    citation correction.
  - Phase Summary table: add F8/F9, S-13, R-11/R-12; show Phase 7
    depending on Phase 5.

================================================================================
6. VERIFICATION YOU MUST RUN BEFORE EDITING
================================================================================

From the repo root:

  rg -n "rain_mode = \"fall\"" src/core/main.rs
  rg -n "perf_preset" src
  rg -n "presentation.rs" implementation-plan.md CLAUDE.md
  ls src/render/engine/presentation
  ls .workflow_state 2>/dev/null; true
  rg -n "pgrep -f matrix-overlay|ps -o pcpu|ps aux" implementation-plan.md tasks.md
  sed -n '28,38p' src/render/engine/pipeline.rs
  sed -n '114,125p' src/core/threads/mod.rs
  sed -n '24,40p' src/render/layout/drawing.rs
  sed -n '101,152p' src/render/engine/presentation/shm.rs
  sed -n '44,57p' src/core/threads/handlers.rs
  sed -n '127,149p' tests/window_integration.rs
  sed -n '42,69p' tests/asd_tests.rs
  sed -n '38,53p' src/ui/gui/mod.rs
  sed -n '31,45p' src/metrics/manager.rs
  rg -n "show_monitor_label|build_logging_enabled" src
  rg -n "fn resize" src/render/engine/presentation
  rg -n "create_collectors|init_collectors" src/metrics

If any finding's evidence is not there, drop that finding and say so.
Do not invent line numbers.

================================================================================
7. DEFINITION OF DONE
================================================================================

The two files, and only those two files, have changed, and:

  [ ] [INTENT] User Objective byte-for-byte unchanged
  [ ] F1–F7, S-01–S-12, R-01–R-10 still exist under those ids
  [ ] F8 (main.rs:28) and F9 (non-rain per-frame cost) exist
  [ ] S-13 exists and is referenced by Phase 9 AC7
  [ ] R-11 and R-12 exist
  [ ] S-03/S-04/S-05/S-06/S-12 measurement text no longer names a command
      that cannot measure the claim
  [ ] §1.6 DAG has Phase 7 depending on Phase 5, not Phase 6
  [ ] §1.6 no longer claims Phases 1–8 are entirely headless
  [ ] Phase 7 has a prerequisite task that removes or gates main.rs:28
  [ ] Every BUILD_RECEIPTS path points at a location the repo can actually
      write
  [ ] No remaining reference in these two files to a flat presentation.rs
  [ ] Default/Medium preset is S-04-capable given S-02, or a branch exists
      that forces target_fps down before Phase 9
  [ ] Task 1.5 does not depend on a Metrics GUI tab that is not shown
  [ ] Task 2.5 names asd_tests.rs and benches/render_bench.rs
  [ ] You appended a short "Audit absorption" note at the bottom of
      implementation-plan.md listing which findings you accepted and which
      you rejected, with the verifying command for each rejection

When finished, print the accepted/rejected list and a diffstat limited to
implementation-plan.md and tasks.md.
```

AMEND — start Phases 1–2 only after the five doc fixes below; then stop and re-gate before Phase 3.

The revised plan is sound enough to execute the instrument pair. It is not sound enough to walk into Phase 3 on the current Phase 5 AC6 text.

---

### Rebuttals

None. All three rejections hold.

- **R-13:** `threads/mod.rs:22-31` is wait-only. libxcb is built for that split. My original row was marked low-confidence; dropping it was correct.
- **Item D:** `git show b15cbc0:implementation-plan.md | grep presentation` is empty. Stale path is only `CLAUDE.md:97` / `:119`. Phase 10.6 is the right home.
- **F-K / factory.rs:** `timer.rs:19` is the only `create_collectors` caller, and nothing calls `timer::`. GL-2 is the stronger statement. Keep it.

The three citation tightenings (six `show_layout`, `shm.rs` at 167/175, label **"HUD CPU"**) are correct.

---

### §3 — S-13 ordering

**Relocate, do not leave it as a single Phase 2 blob.**

`rest_ms` is two different clocks:

| Piece | Needs | Belongs |
|---|---|---|
| `clear` + `rain.update` + metrics glow (6× `show_layout`) | The Phase 2 Cairo surface at 4096×2160 | **Phase 2**, next to the MRC |
| `pre_draw` `GetInputFocus` + `ShmPutImage` + `CreateGc`/`FreeGc`, ×2 monitors | A live X connection | **Phase 1**, on the temporary `cargo run --release` that AC2/AC3 already start |

Phase 2 does not stand up SHM or RandR. Timing present inside the MRC harness would be a new Mock Trap — a present-path number taken off a path production does not take.

Phase 5 AC6 then has **measured** `rain_ms` (MRC) and **measured** `rest_ms` (Phase 1 present + Phase 2 Cairo). Delete the phrase **"or a recorded estimate otherwise"** from Phase 5 AC6. That hatch is Hallucinated Success under a new name. Task 9.6b stays as confirmation, not first contact.

Do not put the Cairo half in Phase 1. Phase 1 has no production-geometry harness and should not grow one.

---

### If Phase 9 AC1 is red and the MRC is green

Replace the five-item list. Rank by “cost the MRC never saw, paid on both CRTCs”:

1. **`Presenter::present` × 2 monitors** — `GetInputFocus` round-trip + `ShmPutImage` of a 4096×2160×4 buffer, then again for 1920×1080. Not in the MRC. This is the floor S-13 exists to name. Check this first.
2. **Metrics glow** — `drawing.rs:27-39`, six `show_layout` per metric per frame, every tick, both windows. Also not in the MRC. After F1 dies this is the next Pango cost centre.
3. **Opaque full-surface `clear()` × 2** — 35 MB write on HDMI-1-0 plus 8 MB on eDP, every frame, before a single glyph.
4. **`rain.update` running when `draw` does not** — `pipeline.rs:32` is not behind the `"fall"` gate. Cheap next to present, free to check, and it is the Pulse-mode leak.
5. **`CreateGc`/`FreeGc` per present, then the `SharedMetrics` lock across both monitors.** Two X requests and a lock are not a 3-point miss by themselves. Look here only after 1–4 have numbers.

Operational suspects, check before any of the above if the reading looks insane:

- Method M-1 attached to the wrong pid (Phase 1 cargo-run still alive, or `pgrep` returned two).
- `target_fps` not actually applied (Ghost Logic repeat; still the 33 ms tick).
- F8 still in place, so a “Pulse” reading is the fall renderer.

---

### What absorption got wrong

Five concrete contradictions. Apply these to `implementation-plan.md` / `tasks.md` before Phase 1 starts. No source edits.

1. **Phase 1 AC2 is unsatisfiable as written.** Method M-1 requires `pgrep -x matrix-overlay` to return exactly one pid. AC2 also forbids killing pid 2462 and starts a second `cargo run --release`, which is another process named `matrix-overlay`. Two pids → AC2 exits 1 by construction. Fix: Phase 1 M-1 takes the cargo-run child pid from the spawn (print it, pin it). Exclude 2462 by recorded pid, not by `pgrep -x` over the whole machine.

2. **Method M-1 was not applied everywhere the old command lived.**
   - `implementation-plan.md` §1.8 layer 4 still says “`ps`/`top` against the deployed binary. Gates S-04, S-05, S-12.”
   - §2.7 first Hallucinated Success row still says “S-03 cross-checks against `ps`.”
   - `tasks.md` 9.7 still says “against `ps`.”
   S-12 is a `cmp`, not a CPU reading; it does not belong in that layer-4 sentence at all.

3. **The §IV citation correction did not finish.**
   - Phase 5 header: “Aligns with concept.md §IV”; Phase 5 Objective still quotes *“1Hz or 0.5Hz is sufficient”* as `concept.md` §IV.
   - Phase 7.2 still quotes `concept.md` §IV for *“No flashing or blinking elements.”*
   Both phrases are `docs/pitfalls.md:70-72`. §1.1 already says this. The phase text was not updated.

4. **S-02 was restated; two older sentences were not.**
   - §1.8 MRC block still: `target: < 20 ms/frame (Phase 3) → < 8 ms/frame (Phase 4)`.
   - §2.3 End State still: rain frame cost `< 8 ms`.
   S-02 is now “hot-path `show_layout` count = 0.” The 8 ms figure is a receipt metric. Those two lines re-open the Phase 4 threshold move AC1 just closed.

5. **Phase 6 still contracts into Phase 7, and AC2 ignores 6.4b.**
   - Forward contract under Phase 6 is still titled “to Phase 7.”
   - 6.6 correctly makes resize conditional on option (a). AC2 still requires “no stale pixels after config change, expose, **or resize**” with no branch.
   - §1.7 rollback trigger is still “S-01…S-12” — S-13 is missing.

Also small: task 1.1 still says “matches `top` semantics.” The campaign just spent a page explaining that `top` is not M-1. Say “matches Method M-1 (% of one core).”

Phase 2 AC2’s “control passes at ~12 ms” has the same profile problem AC1 just shed. Bind it to `--release` and drop the literal 12.

---

Do those five. Then run Phase 1 (instruments + present-path S-13b on the pinned cargo-run pid) and Phase 2 (red MRC + Cairo S-13a). Stop. Re-derive Phase 5’s default `target_fps` from measured numbers, not from the 10-fps placeholder. Do not open Phase 3 on the old arithmetic.