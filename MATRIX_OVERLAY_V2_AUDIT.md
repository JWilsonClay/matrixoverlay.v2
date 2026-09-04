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

---

**GO.** Open Phase 1.

**§2 — keep S-13b in the Phase 1 build.** Do not split it. Wall-clock around `GetInputFocus` / `ShmPutImage` / `CreateGc` does not read `overlay_cpu` or the fps atomic; a fault in 1.1 or 1.4 cannot invent a present-path number. A timing-only binary is a third artifact that still needs the same hooks and a second live run. One constraint on the implementation: accumulate internally and print a summary. Do not log per present — that is a new cost centre on the path being measured.

**Stop condition — F1 is the wrong root cause, not “re-derive the arithmetic,” if any one of these three lands:**

1. **`--release` MRC mean ≤ 20 ms/frame** on a test that still calls production `RainManager::draw` with varying sizes at 4096×2160 (R-06 holds). Then the 750 ms figure was a dev-profile artifact. Phase 3 has nothing to fix that explains 61%. Do not open it. Re-center on `present_ms × fps × 2`.
2. **MRC mean and single-size control within 20% of each other under `--release`.** A-02 is false even if both are slow. Cost is glyph volume or fill, not font-cache eviction. Bucketing will not buy the campaign.
3. **Phase 1 live fps ≥ 15** (frames finishing inside the 33 ms tick) **and** `(present_ms_hdmi + present_ms_edp) × fps ÷ 10 ≥ 40` already. Present × rate × two CRTCs is the 61%. F1 is at most a contributor.

If the MRC stays **> 20 ms**, the control is **several times cheaper**, and live fps is **~1–2**, F1 stands. Take the measured numbers into the Phase 5 identity and continue.

**Diff.** No finding was weakened. Four leftovers, none blocking Phase 1:

- §2.7 Sound Effect row still says S-12 is “mtime against HEAD.” S-12 is `cmp`.
- §1.8 CONTROL still says “costs ~12 ms.”
- §2.4 LOE-3 still says “aligned to `concept.md` §IV”; §2.6 theme 8 still pins “1Hz” on §IV.
- S-01 and Phase 3 AC1 still demand “≥ 40× from ~750 ms.” If the release MRC is 80 ms, that gate is 2 ms and unsatisfiable. Re-gate item, not a Phase 1 hold.

**Phase 1 execution note:** do not use `cargo run --release & TESTPID=$!`. That pins cargo, not the overlay. Build first, run `target/release/matrix-overlay` directly, or parse the binary pid from a startup log / cargo’s children. Sample `/proc/$TESTPID/stat` on that pid.

---

∈ (2, 15) | F1 stands as a defect. A-01 is wrong. **Branch 1 only** — re-derive Phase 5 arithmetic, then open Phase 3. Not X-3-lite. |
| MRC uncalibrated (see b) | Do not honor X-1. Fix the test. Do not open Phase 3. |

Patch §1.9, after the X-3 row, one sentence:

> `fps ∈ (2, 15)` is Branch 1, not a fourth falsifier. It means the live rate is not the inferred 1.3; it does not mean font-cache eviction is absent. Phase 3 still opens if X-1 and X-2 both miss.

Patch §2.5 Branch 1 the same way. Delete the implication “F1 stands unconditionally when A-01 is wrong.” F1 stands in that band only if X-1 and X-2 miss.

## 1. (b) calibration — add Phase 2 AC0. Required before X-1.

X-1 is a fast-green falsifier. R-06 only guards the slow-fake direction (synthetic loop). AC1 currently contradicts X-1: AC1 says “release ≤ 20 ms ⇒ test is wrong, fix the test”; X-1 says “release ≤ 20 ms ⇒ diagnosis is wrong, halt Phase 3.” The only thing that distinguishes those is a calibrated slow run on the same test.

```
Phase 2 AC0 — MRC calibration (binding on X-1)
  Run the identical test_rain_frame_cost_mrc under the dev profile.
  CALIBRATED iff:
    mrc.dev.mean_ms ∈ [500, 900]
    AND mrc.dev.mean_ms ≥ 5 × control.dev.mean_ms
    AND R-06 holds (production RainManager::draw, varying sizes, 4096×2160, realism=4)
  If not CALIBRATED:
    verdict = UNCALIBRATED
    do not honor X-1
    do not open Phase 3
    fix the test (geometry / priming / size variance), do not move the threshold
  X-1 fires only if CALIBRATED AND mrc.release.mean_ms ≤ 20
```

500–900 ms is the investigation anchor on this host, not a universal constant. Landing outside it is “this is not the workload we diagnosed,” not “F1 is false.”

## 2. Receipt schema

Append-only YAML blocks in `receipts/BUILD_RECEIPTS.md`. Units: ms = milliseconds, pct = percent of one core, ticks = `/proc/<pid>/stat` utime+stime ($14+$15).

```yaml
# ---- block: phase 1 ----
phase: 1
git_sha: "<hex>"
binary: "./target/release/matrix-overlay"
host: { nproc: 16, cpu_model: "<lscpu model name>" }
monitors:
  - { name: HDMI-1-0, w: 4096, h: 2160 }
  - { name: eDP,      w: 1920, h: 1080 }
deployed_pid: 2462          # excluded; do not sample
test_pid: <int>             # MUST be the overlay binary, never cargo
m1:
  t0_ticks: <int>
  t1_ticks: <int>
  clk_tck: <int>            # getconf CLK_TCK
  window_s: 300
  cpu_pct: <float>          # 100 * (t1-t0) / clk_tck / window_s
hud_cpu_onscreen: <float>
s03_delta_pp: <float>       # hud_cpu_onscreen - m1.cpu_pct
fps:
  onscreen: <float>
  wallclock_presents_10s: <int>
  wallclock_fps: <float>    # wallclock_presents_10s / 10
present_ms:                 # Instant around each call; accumulate; print once at exit
  HDMI-1-0: { pre_draw: <float>, put_image: <float>, gc: <float>, total: <float>, n: <int> }
  eDP:      { pre_draw: <float>, put_image: <float>, gc: <float>, total: <float>, n: <int> }
x3:
  fps: <float>              # use fps.wallclock_fps
  present_budget_pct: <float>  # (hdmi.total + edp.total) * fps / 10
  fires: <bool>             # fps >= 15 AND present_budget_pct >= 40

# ---- block: phase 2 ----
phase: 2
git_sha: "<hex>"
geometry: { w: 4096, h: 2160, realism: 4, font_size: 16, streams: <int>, distinct_sizes_per_frame: <int> }
mrc:
  dev:     { mean_ms: <float>, p50: <float>, p95: <float>, series: [<40 floats>] }
  release: { mean_ms: <float>, p50: <float>, p95: <float>, series: [<40 floats>] }
control:
  dev:     { mean_ms: <float>, series: [<40 floats>] }
  release: { mean_ms: <float>, series: [<40 floats>] }
cairo_rest_ms: { clear: <float>, rain_update: <float>, glow: <float>, total: <float> }  # 4096×2160, one surface
warmup_ratio: <float>       # release MRC frame40 / frame1
calibration:
  dev_mrc_in_500_900: <bool>
  dev_ratio_vs_control: <float>
  calibrated: <bool>
x1_fires: <bool>            # calibrated AND mrc.release.mean_ms <= 20
x2_fires: <bool>            # abs(mrc.rel - control.rel) / max(mrc.rel, control.rel) <= 0.20
verdict: F1_STANDS | F1_STANDS_REDERIVE | X1 | X2 | X3 | UNCALIBRATED
```

Identity — replace the formula in §1.3 and Phase 5 AC6. Current text multiplies a per-CRTC (or summed) `present_ms` by `monitors` and double-counts present.

```
cpu_pct ≈ (rain_ms + cairo_rest_ms) * fps * monitors / 10
        + (present_ms_hdmi + present_ms_edp) * fps / 10
```

`cairo_rest_ms` is one-surface (MRC harness). Multiply by `monitors`.
`present_ms_*` are already per CRTC. Sum, then scale by fps only. Do not also × `monitors`.
X-3 already uses the second line. AC6 must use both lines.

## 3. Diff `d3f4204 → ab626d9`

§1.9, AC6, AC5, pid correction, leftover four — applied correctly. Nothing weakened.

Two lead-in mismatches left (apply with the patches above, then execute):

1. `tasks.md` 1.7, `tasks.md` AC2 first sentence, `implementation-plan.md` S-13b row, `implementation-plan.md` §1.6 External — still say `cargo run --release`. AC2 body correctly says `cargo build --release` then `./target/release/matrix-overlay`. Change every live-run instruction to that pair. `cargo run` is forbidden for M-1 and S-13b.
2. Phase 5 AC6 / §1.3 identity still does `(rain_ms + cairo_rest_ms + present_ms) × fps × monitors ÷ 10`. Replace with the two-line identity in §2.

No other change required. After those two edits: open Phase 1 against `ab626d9`. Scope remains Phase 1 then Phase 2, then §1.9.

---

```yaml
q1: NO
must_change_before_code: false
open_phase_1_against: d240c80
scope: [phase_1, phase_2, then_§1.9_stop]
q2: END_LOOP
```

**1.** Nothing in `ab626d9 → d240c80` misapplied the round-4 items. Identity, AC0, (2,15) band, `cargo run` purge, receipt schema — all landed.

Two leftover sentences. Neither is a Phase 1 code defect. Patch them when convenient; do not hold the build.

| site | leftover | binding text already is |
|---|---|---|
| `implementation-plan.md` ~L170 | “S-13b therefore rides the temporary `cargo run --release`” | S-13b row + §1.6 + Phase 1 AC2: `cargo build` then `./target/release/matrix-overlay` |
| `implementation-plan.md` A-01 ~L202 | “If Phase 1 shows the live rate is not ~1.3 fps, the F1 diagnosis stands” | §1.9 + §2.5 Branch 1: F1 stands in that band only if X-1 and X-2 miss |

Phase 9 AC7 still says “a Phase 1 `cargo run` still alive.” That names the forbidden form as a suspect. Leave it.

**2.** End the review loop. Remaining questions are measurements:

- AC0 window [500, 900]
- live `fps` vs 1.3 / (2,15) / ≥15
- `present_ms` magnitude
- whether the release MRC is slow

Further reading can only refine thresholds that Phase 1–2 are about to set. The only defect class still findable by reading is a leftover sentence that contradicts a patched AC — finite `rg`, not another round.

```yaml
execute:
  commit: d240c80
  phase: 1
  then: 2
  then: evaluate §1.9 against receipts/BUILD_RECEIPTS.md
  do_not: open Phase 3 in this loop
pid_rule: cargo build --release && ./target/release/matrix-overlay & TESTPID=$!
forbidden: cargo run
m1_phase_1: sample /proc/$TESTPID/stat  # not pgrep -x
```

---

```yaml
round: 6
against: "082cd7b"
phase_2: AUTHORIZED
do_not: [open_phase_3, jump_to_phase_5]
doc_first:
  - "§1.9 cell for fps≥15 ∧ present_budget<40"
  - "X-LIVE rider, threshold 25 ms"
  - "Phase 1 AC1 drop --test metrics_tests"
then: build_MRC
then: "§1.9 stop with rain_ms, cairo_rest_ms, AC0, X-LIVE, X-1, X-2"
```

```yaml
D-1:
  verdict: F1_STANDS_REDERIVE
  new_enum: false
  new_falsifier: false
  cell: "fps >= 15 AND present_budget_pct < 40"
  meaning: >
    Loop is at the hardcoded 33 ms tick. Present is cheap (4.85%).
    Cost is per-frame cairo work × rate — the bin F1 would occupy IF
    it is expensive. This cell does not classify F1. It falsifies A-01
    and forces a Phase 5 re-derive at the measured 30.2 fps.
  receipt: F1_STANDS_REDERIVE
  §1.9_sentence: >
    fps ≥ 15 ∧ present_budget < 40 → Branch 1 at the measured rate;
    X-3 does not fire.

D-2:
  ac0_dev_window: keep_[500,900]
  add: X-LIVE
  threshold_ms: 25
  rule: >
    After the --release MRC exists: if mrc.release.mean_ms > 25
    → verdict UNCALIBRATED_VS_LIVE
    → do not honor X-1
    → do not open Phase 3
    → fix the test, do not move the threshold
  why_25: >
    MRC times one-surface RainManager::draw. Live tick is 19.71 ms
    paying present + second monitor + clear + update + glow.
    A 25+ ms 4K draw cannot coexist with that tick. 18.11 is the
    wrong quantity (two surfaces + rest). 750 ms in a 30.2 fps
    process is physically impossible — that number was the 1.3 fps
    inference talking to itself.
  note: >
    750 ms vs 18.11 ms are not the same measurement
    (dev / draw-only / 4K vs release / full tick / two CRTCs).
    AC0 still tests investigation-identity. X-LIVE tests live-identity.

D-3:
  resequence_now: false
  s04_lever: phase_5
  arithmetic: "19.71 * 1 / 10 = 1.97%  (< 3%)"
  after_phase_2:
    X-1_or_already_under_20: "skip 3–4; Phase 5 next; 3–4 become Extreme-preset quality (same sequel shape as Phase 6)"
    X-2: "F1 wrong mechanism; Phase 5 next"
    X-LIVE: "MRC is not the live path; fix the test; do not open 3"
    slow_and_agrees_with_live: "cannot occur given 19.71 unless the 19.71 reading is wrong"
  why_not_jump: >
    Original order assumed 1.3 fps × 750 ms. Both limbs are dead.
    Temporal control is the S-04 lever. Cheap frames still matter
    for Extreme@30 fps — that is sequel, not the mission gate.
    Do not skip Phase 2: you would set target_fps without rain_ms
    vs cairo_rest_ms. F4 is latent, not live — frames already beat
    the 33 ms cap (threads/mod.rs:116-122).
  do_not_open_phase_3_on: pre_measurement_DAG

D-4:
  action: quarantine
  amend: "Phase 1 AC1 (and Phase 3 AC3) drop --test metrics_tests"
  record: MT-3
  reason: "NvidiaSmiCollector::new_with_command does not exist; file has never compiled; same class as R-11"
  fix_nvidiasmi: false
  blocks_phase_2: false

D-5:
  reject: [a_mutate_config, c_leave_unverified_forever]
  ac2: "accept M-1 59.54% as the CPU number; HUD cross-check = screenshot or 30 s env dump; never touch config.json"
  ac3: "wall-clock 30.2 is the fps"
  follow_up: >
    Always-register FpsCollector in dispatch.rs (instrument, not a
    user metric). Five lines. Ride the Phase 2 commit or a 1.8
    patch. Then a 30 s rerun closes the metric side of AC3.
    Do not hold Phase 2 for it.
```

```yaml
identity_closed: "19.71 * 30.2 / 10 = 59.52  ==  M-1 59.54"
a01: FALSIFIED
x3: false
f4_live: false
phase_2_authorized: true
commit: "082cd7b"
```

---

# AGENT PROMPT — Phase 2 Rework after X-LIVE
# Campaign: 20260903-matrixoverlay-render-remediation
# Against: f5dc741   Branch: refactor/matrixoverlay.v2
# From: Grok Round 7 adjudication
# Scope: Phase 2 rework + plan/task document patches. NO Phase 3. NO Phase 5 code.

```yaml
verdict:
  phase_2: HALTED_REWORK
  phase_3: BLOCKED
  phase_5_code: NOT_THIS_PASS
  s04_lever: phase_5          # 20.64 ms/tick × 1 fps / 10 = 2.06%
  f1_lab: true                # MRC 612.5 vs control 8.55 = 72× — F1 reproduced in cargo-test
  f1_live: false              # production rain.draw 4K 10.00 vs control 8.55 = 1.17×
  identity_closes: true       # 19.43 accounted vs 20.64 observed (6%)
```

Do not implement glyph atlas, bucketing, frame governor, or Pulse Mode.
Do not retune AC0's [500, 900] window.
Do not open Phase 3.

---

## 0. Adjudication (Q1–Q5) — bind these, do not re-argue them

```yaml
Q1:
  instrument: surviving_show_layout_count   # glyphs that PASS the clip guard, not streams×10
  config_source: pinned_literals            # do NOT read ~/.config in the test
  pin:
    rain_speed: 0.1                         # live; default is 1.0 — this was an R-06 miss
    realism: 4
    font_size: 16
    rain_mode: "fall"
    # copy matrix_brightness from the live config.json INTO a literal and record the value in the receipt
  also:
    - rain.update between MRC frames (production does; current harness does not)
    - fix the harness comment that cites production rain.draw as 3.95 ms — receipt 10.0030 wins
  if_glyph_counts_match_and_cost_still_diverges:
    next: MRC-B (one ImageSurface reused, Context fresh per frame, opaque clear per frame)
    not: more priming

Q2:
  x_live: RATIO
  trip: mrc.release.mean_ms / in_process_rain_draw_4k_ms  >=  3.0
  backstop_ms: 25                            # still trips if in-process figure is missing
  current: 612.530 / 10.0030 = 61.2          # trips either form
  do_not_retune_25_after_phase_3

Q3:
  phases_3_and_4: DEMOTED                    # Extreme@30 quality work, same sequel shape as Phase 6
  not: mission_critical
  not: dropped
  next_mission_phase_after_this_rework: 5    # governor + target_fps; S-04 = 2.06% at 1 fps
  phase_3_reentry_only_if:
    live_rain_draw_4k / live_single_size_control_4k  >=  3.0
    both_in_process: true                    # NOT vs cargo-test control 8.55 (Harper)
  today: 10.00 / ~8.6 ≈ 1.17                 # would NOT re-open Phase 3 even after MRC agrees

Q4:
  clear_UNCALIBRATED_VS_LIVE_when:
    - reworked mrc.release.mean_ms / in_process_rain_draw_4k_ms  <  3.0
    - surviving glyph counts explained (ratio recorded)
  that_clears: Phase 2 completion
  that_does_not_clear: Phase 3               # Phase 3 still needs the live ≥3× control test
  ac0_500_900: RETIRED_AS_LIVE_GATE
    keep_as: investigation-identity record
    expect: reworked MRC ~10 ms will FAIL [500,900] — that is correct, do not chase it

Q5_missed:
  - S-13a glow never recorded (drawing.rs:27-39, 6× show_layout). Fill or mark deferred in receipt.
  - pipeline.rs rain.update is NOT behind the "fall" gate — Pulse leak, still live. Do not fix in this pass; record.
  - F8 still live: src/core/main.rs clobber `config.cosmetics.rain_mode = "fall"` after Config::load(). Do not fix in this pass; record.
  - Phase 5 AC6 may consume LIVE terms now (rain_4k=10.003, rain_1080=4.288, clear_x2=3.29, present_x2=1.85). Document only.
```

---

## 1. Standing rules (unchanged)

- Method M-1 only for live CPU. Never `ps -o pcpu`. Never `pgrep -f`.
- Launch: `cargo build --release || exit 1` then `./target/release/matrix-overlay & TESTPID=$!` as TWO statements. Never `cargo run`. Never `cmd && bin &`.
- C-01: 175-line hard cap per module. `rain_manager.rs` is 63. Do not grow it past 175. Glyph-count hook must stay tiny or live in telemetry.
- C-02: no new config fields without `#[serde(default)]`. This pass adds none.
- R-06: production `RainManager::draw`, production-shaped inputs. Defaults are not production-shaped.
- `/nodelete` [INTENT] block is untouchable.
- Do not delete existing S-nn / R-nn / F-nn. Amend. New items continue numbering (F10, R-14, S-14 if needed).
- Do not kill the user's deployed overlay. Temporary binary only.

---

## 2. What to change — files and why

### 2.1 `tests/performance_tests.rs`  (primary)

Current defects:
- `mrc_config()` starts from `Config::default()` → `rain_speed = 1.0` (live is 0.1). R-06 miss.
- `measure_frames` does not call `rain.update` between frames. Production does, every tick.
- No surviving-`show_layout` count.
- Comment at lines 159–162 cites production rain.draw as 3.95 ms. Receipt is 10.0030 ms over 21,854 calls. Fix the comment.

Changeset:

```
mrc_config():
  KEEP starting from Config::default() for unspecified fields (C-02 / deny_unknown_fields).
  PIN, as literals with a comment citing the live file + date:
    cosmetics.rain_speed        = 0.1
    cosmetics.realism           = 4
    cosmetics.rain_mode         = "fall"
    general.font_size           = 16
    cosmetics.matrix_brightness = <copy the live JSON number, write it in the comment>
  Do NOT open ~/.config/matrix-overlay/config.json from the test.

primed_manager():
  Keep the 600-step prime, but prime WITH the pinned rain_speed (0.1).
  Because speed is 10× lower, 600 steps at 33 ms may no longer reach steady state.
  Compute steps from distance: viewport wrap is ~h+400 px; dy = 60 * 0.033 * rain_speed * speed.
  Either raise the step count so mean |stream.y| is inside [0, h], or prime until
  on_screen_fraction is stable across 30 consecutive steps. Record steps used.

measure_frames(rain: &mut RainManager, ...):
  Between frames: rain.update(Duration::from_millis(33), MRC_W, MRC_H, config)
  THEN fresh Context, opaque clear, draw.
  Return (series_ms, series_show_layout_surviving).

NEW: count surviving glyphs inside the draw used by the harness.
  Do not add a log line on the production hot path.
  Preferred: a pub(crate) counter on RainManager incremented only when
  an env or a thread-local is set, reset by the test. See §3.

NEW test (or extend report()):
  print surviving_show_layout mean, p50, and distinct_sizes_that_survived_clip.
  Record both MRC and CONTROL.

NEW test_mrc_b_surface_reuse — ONLY run if glyph counts already match and
  cost still diverges ≥3×. See §3. Do not write it as an always-on gate.
```

S-01 assert (`m < 20`) on `test_rain_frame_cost_mrc`:
- After rework, if the MRC lands near 10 ms this assert will PASS.
- That pass is no longer "the test is wrong" (old AC1 reading) and no longer "F1 is false" (old X-1 reading).
- It means the MRC now agrees with live. Change the assert comment.
- Keep the assert at < 20 ms as a regression rail for "did we accidentally rebuild the 612 ms path."
- X-1 is NOT evaluated until X-LIVE ratio < 3.

### 2.2 `src/render/physics/rain_manager.rs`

Add a surviving-glyph counter that is inert when unset. Must stay well under 175 lines.

```rust
// thread_local so the test can enable it without touching production call sites
thread_local! {
    static COUNT_SHOW: Cell<bool> = const { Cell::new(false) };
    static SURVIVED: Cell<u32> = const { Cell::new(0) };
}

pub fn count_show_layout(enable: bool) { COUNT_SHOW.with(|c| c.set(enable)); SURVIVED.with(|s| s.set(0)); }
pub fn take_survived() -> u32 { SURVIVED.with(|s| s.replace(0)) }
```

In `draw`, only at the existing clip-pass site (line 45), after the continue:

```rust
if COUNT_SHOW.with(|c| c.get()) { SURVIVED.with(|s| s.set(s.get() + 1)); }
```

That is one atomic-free TLS load on the skip path when disabled. Do not allocate. Do not log.

### 2.3 `src/core/telemetry/` + `pipeline.rs`

Already times production `rain.draw` under `MATRIX_OVERLAY_DEBUG_METRICS`. Keep it.
Add surviving-glyph count to that same debug path IF the TLS hook is on, and print means in `telemetry::summary()`. Do not log per frame.

Also instrument an in-process single-size control in the live binary, debug-gated:
- one-shot: if env `MATRIX_OVERLAY_DEBUG_CONTROL=1`, flatten depths to 1.0 for the timed draw only (clone streams, do not mutate the live rain). Record `live_single_size_control_4k_ms`. This is the Phase 3 re-entry denominator. Optional this pass; required before anyone argues Phase 3 open.

### 2.4 `implementation-plan.md` + `tasks.md`  (documents only)

Amend, do not rewrite history.

```
§1.9:
  Add cell: AC0 pass ∧ X-LIVE trip = UNCALIBRATED_VS_LIVE
            (lab F1 real, live path disagrees; halt Phase 3; rework MRC)
  Change X-LIVE from absolute 25 ms to ratio ≥ 3 against in-process 4K rain.draw
  Keep 25 ms as backstop when the in-process figure is absent
  State: fps ≥ 15 ∧ present_budget < 40 remains F1_STANDS_REDERIVE (already patched)

§2.5:
  After this rework completes (ratio < 3 OR ratio still ≥ 3 with glyph counts explained):
    next mission phase is Phase 5
    Phases 3–4 demoted to sequel (Extreme@30 quality), opened only if
      live_rain_draw_4k / live_single_size_control_4k ≥ 3

Phase 2 tasks:
  Add 2.8 Phase 2 rework (this prompt)
  AC0 [500,900] stays as investigation record; it is no longer a live gate
  New AC: X-LIVE ratio
  New AC: surviving show_layout printed for MRC and CONTROL and live

Phase 5 AC6:
  Note that LIVE terms already exist and may be used:
    rain_4k=10.0030  rain_1080=4.2883  clear_x2=3.29  present_x2=1.8473
    ms_per_tick=20.64  at 1 fps → 2.06%
  Do not implement Phase 5.

Receipt schema:
  Add:
    x_live: { mrc_ms, live_rain_4k_ms, ratio, threshold: 3.0, tripped }
    glyphs: { mrc_surviving_mean, control_surviving_mean, live_surviving_mean }
    live_control_4k_ms: <optional>
```

### 2.5 Do NOT touch this pass

- `src/core/main.rs` F8 clobber (record only)
- `pipeline.rs` rain.update gate (record only)
- `drawing.rs` glow (S-13a fill OR mark deferred)
- glyph_cache.rs / atlas / governor / presets / Pulse
- user's `~/.config/matrix-overlay/config.json`
- AC0 numeric window

---

## 3. Pseudocode

### 3.1 Decisive experiment (always run)

```text
GIVEN pinned literals (rain_speed=0.1, realism=4, font_size=16, rain_mode=fall)
      prime until on-screen fraction stable
      enable COUNT_SHOW

FOR each of 40 frames:
    rain.update(33ms, 4096, 2160, config)     # production does this
    fresh Context + opaque Source-black clear
    t0 = Instant::now()
    rain.draw(...)
    dt_ms = elapsed
    survived = take_survived()

RECORD mean_ms, series, survived_mean, distinct_sizes_that_drew

COMPARE
    live_rain_4k_ms     = 10.0030          # already measured, 21854 calls
    mrc_release_ms      = <this run>
    ratio               = mrc_release_ms / 10.0030
    x_live_trips        = ratio >= 3.0 OR mrc_release_ms > 25

IF survived_mean_MRC ≈ survived_mean_LIVE
   AND ratio >= 3:
    RUN 3.2 MRC-B
ELSE IF survived_mean_MRC >> survived_mean_LIVE:
    verdict = CLIP_GUARD / y-distribution
    fix = priming + rain_speed pin (already done); re-run once
ELSE IF ratio < 3:
    verdict = X_LIVE_CLEARED
    Phase 2 may complete
    Phase 3 stays closed unless live/control ≥ 3 in-process
```

### 3.2 MRC-B — cheaper third experiment (only if counts match and ratio still ≥ 3)

```text
surface = ImageSurface::create(ARgb32, 4096, 2160)   # ONCE, reused
FOR 40 frames:
    rain.update(...)
    cr = Context::new(&surface)                      # fresh ctx, same surface
    opaque clear
    rain.draw(...)

IF mrc_b_ms drops toward ~10:
    cause = scaled-font cache dying on surface recreate
    (current harness already reuses one surface — if this is already the case,
     MRC-B will NOT drop, and the cause is SHM-backed vs standalone ImageSurface)
IF mrc_b_ms stays ~612:
    next experiment (NOT this pass unless cheap):
      draw onto an SHM-backed ImageSurface without X present
      OR accept that cargo-test Pango cache ≠ production process cache
      and stop using cargo-test as the F1 magnitude source
```

Current harness already creates one surface and reuses it (`measure_frames` line 154).
MRC-B is therefore "confirm we did not regress to per-frame surface create."
If the 612 ms figure is already on a reused surface, skip MRC-B and record:

```
surface_reused: true
mrc_b: not_applicable
next_cause_class: process_or_shm_vs_standalone
```

### 3.3 Live glyph count (30–60 s, debug env, do not mutate user config)

```text
MATRIX_OVERLAY_DEBUG_METRICS=1
# enable COUNT_SHOW from an env the binary already keys on, or a new
# MATRIX_OVERLAY_DEBUG_GLYPHS=1 that only sets the TLS flag at startup

run ./target/release/matrix-overlay directly for 60 s after t>=120 s warm-up
print in exit summary:
  rain_draw_4k_ms
  rain_draw_1080_ms
  survived_show_layout_4k_mean
  survived_show_layout_1080_mean
```

Compare those survived means to the MRC survived mean. That is the 4.5 experiment.

---

## 4. Receipt fields to append

```yaml
phase: 2.8
git_sha: "<this commit>"
mrc:
  release: { mean_ms: _, series: [], survived_show_layout_mean: _, distinct_sizes_drew: _ }
  dev:     { mean_ms: _, survived_show_layout_mean: _ }   # record, do not gate
control:
  release: { mean_ms: _, survived_show_layout_mean: _ }
config_literals:
  rain_speed: 0.1
  realism: 4
  font_size: 16
  matrix_brightness: _
  prime_steps: _
live:
  rain_draw_4k_ms: 10.0030          # unless re-measured
  rain_draw_1080_ms: 4.2883
  survived_show_layout_4k_mean: _
  survived_show_layout_1080_mean: _
x_live:
  ratio: _                          # mrc.release.mean_ms / live.rain_draw_4k_ms
  threshold: 3.0
  backstop_ms: 25
  tripped: _
verdict: X_LIVE_CLEARED | UNCALIBRATED_VS_LIVE | CLIP_GUARD | PROCESS_CACHE
phase_2_complete: <true only if ratio < 3>
phase_3: BLOCKED                    # unless live/control_in_process >= 3
```

---

## 5. Definition of done for this pass

Checkable:

- [ ] `mrc_config()` pins `rain_speed = 0.1` and the other literals. Zero reads of `~/.config`.
- [ ] `measure_frames` calls `rain.update` between frames.
- [ ] Surviving `show_layout` count printed for MRC, CONTROL, and (if run) live.
- [ ] Harness comment no longer says production rain.draw is 3.95 ms.
- [ ] X-LIVE in plan + tasks is a ratio ≥ 3 against in-process 4K rain.draw, 25 ms backstop kept.
- [ ] AC0 [500,900] labeled investigation-identity only. Not retuned.
- [ ] §2.5 / Phase 2 forward contract: next mission phase is Phase 5. 3–4 demoted.
- [ ] Phase 3 re-entry criterion written: live/control in-process ≥ 3. Not cargo-test control.
- [ ] F8, ungated `rain.update`, missing S-13a glow recorded as known-open. Not "fixed."
- [ ] C-01 honored (`wc -l` rain_manager.rs ≤ 175).
- [ ] User config.json byte-identical.
- [ ] Receipt block 2.8 appended.
- [ ] No Phase 3 source. No Phase 5 source. No atlas.

Stop. Bring the 2.8 receipt. Do not open Phase 3. Do not jump to Phase 5 code in this commit.
```

---

# AGENT PROMPT — Round 8: Close Phase 2, Open Phase 5
# Campaign: 20260903-matrixoverlay-render-remediation
# Against: fbcc57d   Branch: refactor/matrixoverlay.v2
# From: Grok Round 8 adjudication of PHASE_2.8_RECEIPT.md
# Scope: document-gate rewrite + optional 2.9 footnote + Phase 5 ONLY.

```yaml
round: 8
against: fbcc57d
q1: RETIRE_MRC_AS_GATE          # keep test, label LAB_F1
q2: OPTIONAL_2_9_FOOTNOTE       # gtk::init + font_options dump; cannot block Phase 5
q3: PHASE_5_AUTHORIZED
phase_2: CLOSED_LAB_DIVERGENT
phase_3: BLOCKED_AND_DEMOTED
phase_4: BLOCKED_AND_DEMOTED
phase_5: OPEN
do_not: [phase_3_source, phase_4_source, atlas, pulse, presets, deploy, retune_AC0_500_900, mutate_user_config]
```

Do not re-argue PROCESS_CACHE. Do not spend a phase making cargo-test match live.
Do not pull 605 ms into any Phase 5 number.

---

## 0. Rulings — bind these

```yaml
Q1:
  chase_process_cache_as_gate: false
  reason: >
    MRC existed to open/block Phase 3. Phase 3 is demoted on an in-process
    1.25× that needs no MRC. An MRC that cannot reproduce live cost is a
    lab curiosity, not a campaign instrument. Keeping it as a halt is a
    Mock Trap of the campaign itself.
  keep_the_test: true
  label: LAB_F1
  meaning: "cargo-test reproduces Pango size-churn; the overlay process does not"

Q2:
  worth_one_cheap_pass: true
  block_phase_5: false
  experiment: 2.9 sidecar — see §2
  leading_mechanism: >
    gtk::init() / PangoCairoFontMap / Xft font options in the overlay
    process vs a bare cargo-test font map. Not merely "SHM vs ImageSurface."

Q3:
  phase_2_complete_false_blocks_phase_5: false
  rewrite_the_gate: true
  phase_2_mission_deliverables: COMPLETE
    - F2 overlay_cpu fix, verified ±1 pp (AC2 1.8)
    - fps instrument, verified ±10% (AC3 1.8)
    - S-13b present_ms per CRTC
    - Mock Trap deleted
    - S-13a clear + rain.update (glow still unmeasured — recorded)
    - live identity closes 19.43 vs 20.64 (6%)
    - Phase 3 re-entry measured: 1.25 < 3.00
  x_live_remaining: finding_not_halt
    # "the cargo-test MRC is not the live path" — now known, not blocking
  next: Phase 5
```

Live terms Phase 5 AC6 MUST use. Do not mix MRC milliseconds into them.

```yaml
live:
  rain_4k_ms: 9.6220          # run A, 12691 calls; figure of record 10.0030 agrees within 4%
  rain_1080_ms: 4.2344
  live_control_4k_ms: 7.3164  # in-process single-size
  live_over_control_4k: 1.25
  present_ms_sum: 1.6160
  ms_per_tick: 20.64
  fps_now: 30.2
  m1_cpu_pct: 60.4613
  s04_at_1fps: 2.06           # 20.64 × 1 / 10
  s04_gate: 3.0
  default_target_fps: 1       # 20.64 × fps / 10 < 3 → fps <= 1.45
```

---

## 1. Standing rules

- Method M-1 only. Never `ps -o pcpu`. Never `pgrep -f`.
- Launch as TWO statements:
    cargo build --release || exit 1
    ./target/release/matrix-overlay & TESTPID=$!
  Never `cargo run`. Never `cmd && bin &`.
- C-01: 175-line hard cap. Tick module and config modules included.
- C-02: `target_fps` MUST have `#[serde(default = "default_target_fps")]`.
  Existing `~/.config/matrix-overlay/config.json` must parse unchanged.
- C-05 / pitfalls.md:70-72: 1 Hz is sufficient; no flashing. Default 1 fps follows this.
- C-06: do not deploy, do not touch autostart, do not kill the user's overlay.
- R-06: do not let Phase 5 tests measure a path production does not take.
- `/nodelete` [INTENT] is untouchable.
- Amend, do not delete, S-nn / R-nn / F-nn. New items continue numbering.
- F8 (`main.rs` rain_mode clobber), ungated `rain.update`, unmeasured glow: record only. Do not fix.

---

## 2. First commit — gate rewrite + optional 2.9 (no Phase 5 behavior yet)

### 2.1 `implementation-plan.md` + `tasks.md`

```
§1.9:
  X-LIVE remaining open is a FINDING: cargo-test MRC ≠ live path.
  It no longer halts the campaign.
  New verdict: PHASE_2_CLOSED_LAB_DIVERGENT
  MRC test stays. Relabel every "validation / S-08 gate / Phase 3 red-before-green"
  that treated the cargo-test MRC as the live cost. It is LAB_F1.

§1.6 / §2.5 / Phase 2 forward contract:
  Phase 2 mission deliverables are COMPLETE.
  Next mission phase is Phase 5.
  Phases 3–4 remain demoted sequels.
  Phase 3 re-entry is unchanged: live_rain_4k / live_single_size_4k ≥ 3.0,
  both in-process. Currently 1.25. Do not reopen.

Phase 2 status banner in tasks.md:
  STATUS: CLOSED_LAB_DIVERGENT (2026-09-03, fbcc57d + this patch)
  X-LIVE ratio 60.55 is recorded, not blocking.

Phase 5 AC6:
  Consume LIVE terms only (block in §0).
  Identity:
    cpu_pct ≈ ms_per_tick × target_fps / 10
    with ms_per_tick = 20.64 measured at 30.2 fps
  Projection at default 1: 2.06% < 3%.
  If a later measurement of ms_per_tick at the new rate disagrees by >20%,
  re-measure rather than trust the 30 fps tick cost at 1 fps
  (clear + present may not scale perfectly; rain.draw should).

S-01 / S-08:
  S-01's 20 ms cargo-test gate is no longer a campaign halt.
  Keep the test as LAB_F1 documentation.
  S-08 "red before Phase 3" is vacated because Phase 3 is not opening.
```

### 2.2 Optional 2.9 — one cheap pass, cannot block §3

If this takes more than ~30 minutes, skip and write the pitfalls stub instead.

```text
E1  In test_rain_frame_cost_mrc, call gtk::init() once at the start.
    Re-run: cargo test --release --test performance_tests test_rain_frame_cost_mrc -- --nocapture
    Record mean_ms. If it drops toward ~10: PROCESS_CACHE = fontmap/gtk.
    If it stays ~600: not gtk::init alone.

E2  From a 15 s MATRIX_OVERLAY_DEBUG_METRICS run, print
    cairo_surface_get_font_options on the SHM surface.
    From the MRC, print the same on the standalone ImageSurface.
    One receipt row. Do not retune the harness to force a match.

E3  Phase 10 pitfalls stub (write the text now, even if E1/E2 skipped):
    "Pango size-churn costs 74× inside cargo-test and 1.25× inside the
     overlay process. Leading remaining mechanism: gtk::init() /
     PangoCairoFontMap / Xft options vs a bare test font map — not glyph
     volume, not the clip guard, not rain_speed. The cargo-test MRC is a
     lab reproduction of F1, not a measurement of the live path."
```

Receipt 2.9 is append-only. `phase_5_blocked_by_2_9: false` in all cases.

---

## 3. Phase 5 — Frame Governor (the mission lever)

Objective: the hardcoded 33 ms tick is why the process is at 60%. Frames already finish in 20.64 ms, so F4 fail-open is latent, not live — but the interval itself is the cost. Add `target_fps`, honor it, project S-04.

### 3.1 Tasks (from the existing Phase 5 list — execute these, nothing else)

```
5.1  Fix spawn_tick_thread in src/core/threads/mod.rs (~L114-125).
     Today:
       let interval = Duration::from_millis(33);
       if elapsed < interval { sleep(interval - elapsed); }
       else { sleep(1ms); }          # F4 fail-open
     Time spent blocked in send() on the bounded(1) channel is counted
     in elapsed, so a slow frame sleeps 1 ms and immediately re-queues.

5.2  Replace the sleep-accumulator with a monotonic deadline:
       let period = Duration::from_secs_f64(1.0 / target_fps.max(1) as f64);
       let mut deadline = Instant::now() + period;
       loop {
         // work
         let now = Instant::now();
         if now < deadline { sleep(deadline - now); }
         else { /* skip missed ticks; do not queue catch-up frames */ }
         deadline += period;
         if deadline < Instant::now() { deadline = Instant::now() + period; }
       }
     Never sleep 1 ms as a fallback. Never fold send() block time into
     the period math — sample Instant after the send returns.

5.3  Add general.target_fps: u32
       types.rs:   #[serde(default = "default_target_fps")]
       defaults.rs: pub fn default_target_fps() -> u32 { 1 }
     Default is 1, not 10. The pre-audit 10 was a placeholder the budget
     identity was expected to lower. Live identity lowered it to 1.
     pitfalls.md:72 already sanctions 1 Hz.

5.4  Clamp on load to 1..=60. Zero must not divide. Absurd must not
     recreate the runaway.

5.5  Expose target_fps in the GUI General tab using the existing
     widget / update_config_from_widgets pattern in ui/gui/logic.rs.
     Do not invent a new tab.

5.6  Unit test: inject a simulated 200 ms frame; assert the next tick
     is not issued before the configured period. This is S-07 / F4.
```

### 3.2 Acceptance criteria

```
AC1  S-07: injected 200 ms frame → tick never re-queues faster than period.
AC2  Measured fps (Phase 1 instrument) tracks configured target_fps within ±10%
     on a 60 s direct-binary run at target_fps=1 and at target_fps=5.
     Use MATRIX_OVERLAY_DEBUG_METRICS=1 exit summary + wall-clock presents.
     Do not mutate the user's config.json — pass target_fps via a test config
     copy, an env override if one exists, or a temp XDG_CONFIG_HOME.
AC3  Clamp: 0 → 1, 9999 → 60, verified by unit test.
AC4  C-02: user's real config.json (no target_fps field) loads and defaults to 1.
AC5  HITL — user visual sign-off that 1 fps rain is smooth enough / non-strobing.
     Mark AC5 BLOCKING_HITL. Do not fake it. Do not skip silently.
     Record "pending user" if the user is not in the loop this commit.
AC6  Budget identity, LIVE terms only:
       cpu_pct ≈ 20.64 × target_fps / 10
       at default 1 → 2.06% < 3%
     Then MEASURE with Method M-1 over ≥ 120 s after warm-up (t ≥ 120 s
     from the 12-min run: CPU rises until streams fill, then flats).
     Gate: M-1 < 3% of one core at target_fps=1.
     If M-1 ≥ 3% at 1 fps, do not declare S-04 met and do not invent a
     Phase 3 re-open to explain it — bring the receipt back.
```

### 3.3 Pseudocode — tick thread

```rust
// src/core/threads/mod.rs  — replace the 33 ms block
fn spawn_tick_thread(..., target_fps: u32, ...) {
    let fps = target_fps.clamp(1, 60);
    let period = Duration::from_nanos(1_000_000_000 / fps as u64);
    thread::spawn(move || {
        let mut deadline = Instant::now() + period;
        loop {
            if shutdown.load(Ordering::Relaxed) { break; }
            // send must NOT be inside the period measurement
            let _ = tick_tx.try_send(());      // or send, but timestamp AFTER
            let now = Instant::now();
            if now < deadline {
                thread::sleep(deadline - now);
            }
            // skip missed ticks; do not catch up
            deadline += period;
            while deadline < Instant::now() {
                deadline += period;
            }
        }
    });
}
```

`target_fps` must be readable by the tick thread. Today the interval is a local `33`.
Pass it in at spawn from the loaded Config. If config can change at runtime via the GUI,
the thread needs a way to see the new value (AtomicU32 is enough; do not take the
SharedMetrics mutex on this path).

### 3.4 What Phase 5 does not do

- Does not touch rain_manager.rs draw loop.
- Does not bucket fonts, does not build an atlas.
- Does not implement Pulse Mode, does not remove F8.
- Does not gate rain.update behind `"fall"` (record only).
- Does not measure glow (record only).
- Does not run `scripts/install.sh`, does not copy to `~/.local/bin`.
- Does not use cargo-test MRC milliseconds in any AC.

---

## 4. Receipt schema — append

```yaml
# ---- block: phase 2 close ----
phase: 2
status: CLOSED_LAB_DIVERGENT
x_live: { ratio: 60.55, finding_not_halt: true }
lab_f1: { mrc_ms: 605.684, control_ms: 8.176, ratio: 74 }
live_f1: { rain_4k_ms: 9.6220, control_4k_ms: 7.3164, ratio: 1.25 }
phase_3: BLOCKED_AND_DEMOTED
optional_2_9:
  gtk_init_mrc_ms: _          # or "skipped"
  font_options_live: _
  font_options_mrc: _
  conclusion: _

# ---- block: phase 5 ----
phase: 5
git_sha: _
target_fps_default: 1
ac1_governor_holds: _
ac2:
  target_1: { wallclock_fps: _, metric_fps: _, m1_cpu_pct: _, window_s: _ }
  target_5: { wallclock_fps: _, metric_fps: _ }    # tracking only; not an S-04 gate
ac3_clamp: _
ac4_old_config_loads: _
ac5_user_signoff: pending | accepted | rejected
ac6:
  projected_pct_at_1: 2.06
  measured_m1_at_1: _
  s04: MET | UNMET
verdict: S04_MET | S04_UNMET_BRING_RECEIPT
```

---

## 5. Definition of done

- [ ] Plan + tasks rewritten: Phase 2 CLOSED_LAB_DIVERGENT, MRC = LAB_F1, X-LIVE is a finding, Phase 5 next.
- [ ] Phase 3 re-entry still in-process ≥ 3. Not cargo-test control. Not silently reopened.
- [ ] Optional 2.9 done or explicitly skipped with the pitfalls stub written.
- [ ] `target_fps` exists, default 1, clamped 1..=60, `#[serde(default)]`.
- [ ] Tick honors `target_fps` with a monotonic deadline. F4 1 ms fallback gone.
- [ ] Unit test covers the 200 ms injection.
- [ ] User config.json byte-identical; loads; defaults target_fps to 1.
- [ ] AC2/AC6 live run: direct binary, pinned pid, M-1 at target_fps=1, t0 after warm-up.
- [ ] AC5 marked pending-user if not signed off. Not forged.
- [ ] Receipt blocks appended.
- [ ] C-01: every touched file `wc -l` ≤ 175.
- [ ] No Phase 3/4 source. No atlas. No Pulse. No deploy.
- [ ] No 605 ms figure used as a Phase 5 input.

Stop after the Phase 5 receipt. If AC6 M-1 ≥ 3% at 1 fps, stop and bring the receipt — do not open Phase 3 to explain it.
```

---

```yaml
round: 9
against: 4b6cb16
s04: UNMET
accept_3.0166: false
reopen_phase_3: false
phase_5_8: OPEN
phase_5_9: GATED_ON_COLLECTORS
phase_7: BLOCKED_UNTIL_FLOOR
prompt: /home/workdir/artifacts/ROUND9_AGENT_PROMPT.md
```

```yaml
ruling: >
  0.017 pp is located, not mysterious. 1.943% is rain+present.
  The 1.073% residual still contains per-tick work (clear, glow).
  That is not a pure floor. Stop spending against a subtraction.
  target_fps is already 1 and cannot cut either half.
  ~5 ticks in 300 s is one nvidia-smi spawn. Measure, then decide.
next: isolate glow, clear@1fps, collectors, remainder
then: raise update_ms IFF collectors dominate
s05: conditional — if true floor ≥ 0.5%, Phase 7 cannot meet <0.5%
ac5: pending HITL
standing_rule: assert load behavior (N events, achieved rate), never one step
```

Do not declare S-04 met by rounding 3.0166 down. Agent executes the file.

# AGENT PROMPT — Round 9: Isolate the 1.07% residual
# Campaign: 20260903-matrixoverlay-render-remediation
# Against: 4b6cb16   Branch: refactor/matrixoverlay.v2
# From: Grok Round 9 adjudication of PHASE_5_RECEIPT.md
# Scope: plan/tasks patch + Phase 5.8 floor measurement. Then a gated 5.9 IFF collectors dominate.

```yaml
round: 9
against: 4b6cb16
s04: UNMET                    # 3.0166 vs 3.0 — do not reclassify as pass
phase_3: BLOCKED_AND_DEMOTED  # 1.25 < 3.00; rain is not why S-04 misses
phase_5_governor: COMPLETE
phase_5_8: OPEN               # isolate residual
phase_5_9: GATED              # raise update_ms only if 5.8 says collectors dominate
phase_6: NOT_THIS_PASS
phase_7: NOT_THIS_PASS        # S-05 is now conditional on the true floor
do_not: [accept_3.0166, move_the_gate, reopen_phase_3, atlas, pulse, presets, deploy, forge_AC5]
```

---

## 0. Rulings — bind these

```yaml
miss:
  m1_at_1: 3.0166
  gate: 3.0
  delta_pp: 0.0166
  delta_ticks_300s: ~5              # CLK_TCK=100; one nvidia-smi spawn can do this
  identity_closes: "1.943 + 1.073 = 3.016 vs M-1 3.0166"

the_1_073_is_not_a_pure_floor:
  accounted_render_ms: 19.4348      # rain_4k + rain_1080 + present ONLY
  still_inside_the_residual:
    clear_x2_ms: ~3.29              # S-13a, measured at 30 fps; per-tick = RATE-DEPENDENT
    glow: unmeasured                # drawing.rs:27-39, 6× show_layout per metric per frame; RATE-DEPENDENT
  true_floor_candidates:
    - metrics collectors on update_ms (nvidia-smi subprocess)
    - GTK / tray thread
    - XCB event thread
  consequence: >
    Stop spending against a subtraction. Split residual into
    (clear + glow) which belong in the render identity, and
    (collectors + GTK + XCB) which is the true floor.
    target_fps is already 1 — the governor cannot cut either half further.

routes_rejected:
  accept_now: false                 # gate stays 3.0 until the residual is named
  reopen_phase_3: false
  lower_target_fps: false           # clamp is 1; pitfalls.md:72 floor
  full_phase_6: false               # damage/mutex architecture is not the next lever

routes_authorized:
  5.8: measure the residual directly
  5.9: raise update_ms IFF 5.8 shows collectors dominate
  then: re-run M-1 at target_fps=1
  if_still_over: written exception against concept.md §III, then AC5 + later phases

s05_conditional: >
  Pulse Mode S-05 is <0.5%. If the true floor stays ≥0.5%, Phase 7 cannot
  meet S-05 without cutting collectors. Do not open Phase 7 until 5.8 numbers
  exist and either the floor is under 0.5% or S-05 is explicitly amended.

standing_rule: >
  A performance AC must assert behaviour under load (N events, achieved rate),
  never a property of one step. Recurrences: Phase 1 format_overlay_cpu local
  copy; Phase 2 Config::default() substitution; Phase 5 next_deadline now+1ms
  passing S-07. Write this into tasks.md AND docs/pitfalls.md.
```

Live terms — do not mix MRC 605 ms into any number.

```yaml
live_at_1_fps:
  wallclock_fps: 0.995
  metric_fps: 1.009
  m1_cpu_pct: 3.0166
  rain_4k_ms: 10.5461
  rain_1080_ms: 5.6787
  present_sum_ms: 3.2100
  render_subtotal_ms: 19.4348
  render_pct: 1.943
  residual_pct: 1.073
```

---

## 1. Standing rules (unchanged)

- Method M-1 only. Two `/proc/<pid>/stat` samples, fields 14+15, after known restart.
- Warm-up: t0 after streams have filled (the 12-min run flattened at t≈120 s; Phase 5 used 300 s warm-up — keep that).
- Launch as TWO statements:
    cargo build --release || exit 1
    ./target/release/matrix-overlay & TESTPID=$!
  Never `cargo run`. Never `cmd && bin &`.
- Throwaway `$HOME` / `XDG_CONFIG_HOME` for test runs. User config.md5 must stay `4747e9c8a1bb239170f3a446d083a4e6`.
- C-01: 175-line hard cap.
- C-02: no new config fields without `#[serde(default)]`. `update_ms` already exists — 5.9 only changes the default and/or the live test-run value.
- C-06: no deploy, no autostart, no kill of a user overlay.
- `/nodelete` [INTENT] untouchable.
- Amend, do not delete, S-nn / F-nn / R-nn.
- F8, ungated `rain.update`: record only.

---

## 2. First commit — documents + standing rule

`implementation-plan.md` + `tasks.md` + `docs/pitfalls.md`:

```
§1.3 budget identity — three lines, not two:

  cpu_pct ≈ (rain_ms + cairo_rest_ms) × fps × monitors / 10     # per-surface
          + (present_hdmi + present_edp) × fps / 10            # per-CRTC
          + floor_pct                                          # NEW: collectors + GTK + XCB

  cairo_rest_ms must eventually include clear + glow.
  floor_pct is frame-rate-independent. target_fps cannot cut it.

§1.9 / Phase 5 status:
  AC6 UNMET by 0.017 pp. Residual located as a subtraction, not as rain.
  Phase 3 stays demoted.

New standing rule (pitfalls.md + tasks.md header):
  "A performance AC must assert behaviour under load (N events, achieved
   rate), never a property of one step. A one-step assertion will pass a
   fail-open implementation. Three recurrences in this campaign."

S-05 footnote:
  Conditional on true floor. Do not open Phase 7 until 5.8 reports.
```

Do not move the 3.0% number.

---

## 3. Phase 5.8 — isolate the residual (this pass's work)

Instrument. Do not "fix" glow, do not cache the metrics panel, do not raise `update_ms` until the numbers exist.

### 3.1 Four terms, one 300 s window after warm-up, target_fps=1

```text
F-A  GLOW — wall time of draw_metrics / the 6× show_layout path
     (drawing.rs:27-39), both CRTCs, accumulate internally, print once
     at exit next to the S-13b summary. Same constraint as S-13b:
     no per-frame log line on the measured path.
     Env-gate: MATRIX_OVERLAY_DEBUG_METRICS already exists — extend it.

F-B  CLEAR — wall time of Renderer::clear() both CRTCs at 1 fps.
     S-13a has ~3.29 ms at 30 fps on one surface ×2. Re-measure at 1 fps
     on the live SHM surfaces. Do not reuse the 30 fps figure as if it
     were the 1 fps figure.

F-C  COLLECTORS — over the SAME M-1 window:
       - overlay utime+stime (already M-1)
       - count of nvidia-smi processes spawned / wait time
         (wrap the spawn site or read /proc/<pid>/task/*/children + comm)
       - if cheap: time the collector tick in metrics/manager.rs the
         same way rain.draw is timed (accumulate, print once)
     CLAUDE.md already warns nvidia-smi wakes the sleeping dGPU.

F-D  REMAINDER = M-1_pct - render_pct - glow_pct - clear_pct - collector_pct
     This is GTK/tray + XCB event thread + anything still unnamed.
```

Identity after 5.8 must be four named terms plus remainder, not a subtraction called "floor."

### 3.2 Implementation constraints

- Accumulate internally, one summary at exit. No per-frame log.
- Prefer extending `src/core/telemetry/` (already at 157 / 99 lines — C-01: if growth would exceed 175, new file under telemetry/).
- `drawing.rs` / `pipeline.rs` stay under 175.
- Debug-gated. Production path unchanged when env is unset.
- OnceLock for env flags already exists (`Renderer::debug_flags`) — use it. Do not call `env::var_os` per frame.

### 3.3 Live run

```
cargo build --release || exit 1
XDG_CONFIG_HOME=<throwaway> MATRIX_OVERLAY_DEBUG_METRICS=1 \
  ./target/release/matrix-overlay & TESTPID=$!
# confirm exe, comm, target_fps=1 (default)
# warm-up ≥ 120 s (prefer 300 s to match Phase 5)
# M-1 window 300 s
# print telemetry summary on SIGTERM
```

Record raw ticks, not only percentages.

### 3.4 Decision table — evaluated AFTER the receipt exists

```yaml
if glow_pct + clear_pct >= 0.5:
  next: surgical metrics-panel cache (old Phase 6.2/6.3 only)
        NOT full Phase 6, NOT Phase 3
        NOT this commit — bring the receipt

if collector_pct >= 0.4 OR nvidia-smi wait dominates F-C:
  next: Phase 5.9 in the SAME campaign pass is authorized
        change default update_ms 2000 → 5000
        #[serde(default)] already on the field; existing configs keep 2000
        unless you also write a one-line note that the campaign test run
        sets 5000 in the throwaway config
        then re-run M-1 at target_fps=1

if remainder_pct >= 0.5 AND collectors are small:
  next: ACCEPT 3.02% with a written exception against concept.md §III
        "structural floor = GTK + XCB + collectors; 20× down from 60.7%"
        then AC5 HITL and stop

if after 5.9 M-1 < 3.0:
  s04: MET
  then AC5 HITL

if after 5.9 M-1 still >= 3.0:
  written exception, do not chase further this campaign
```

Do not implement 5.9 until F-A..F-D numbers are in the receipt. If they are in and collectors dominate, 5.9 may ride a second commit in this pass.

---

## 4. Phase 5.9 — gated, collectors only

```
defaults.rs / the existing update_ms default: 2000 → 5000.
Rationale: 2 s polling of nvidia-smi wakes a sleeping dGPU and is
invisible-quality at 5 s for a temperature readout.
Do not change the render path.
Do not change target_fps.
Re-run the 5.8 live protocol at update_ms=5000, target_fps=1.
```

If you do not have F-C evidence that collectors dominate, do not do 5.9.

---

## 5. Receipt schema

```yaml
phase: 5.8
git_sha: _
target_fps: 1
update_ms: 2000                  # 5000 if 5.9 ran
m1:
  t0_ticks: _
  t1_ticks: _
  clk_tck: _
  window_s: 300
  cpu_pct: _
render:
  rain_4k_ms: _
  rain_1080_ms: _
  present_sum_ms: _
  pct: _
clear:
  hdmi_ms: _
  edp_ms: _
  sum_ms: _
  pct: _                         # sum_ms × fps / 10
glow:
  hdmi_ms: _
  edp_ms: _
  sum_ms: _
  pct: _
collectors:
  nvidia_smi_spawns: _
  nvidia_smi_wait_s: _
  collector_tick_ms_mean: _
  pct: _                         # best available: wait_s/window_s*100 or timed tick
remainder_pct: _                 # m1 - render - clear - glow - collectors
decision: 5.9_UPDATE_MS | PANEL_CACHE | ACCEPT_3_02 | S04_MET
s05_floor_ok: _                  # true iff (remainder + collectors) < 0.5
ac5_user_signoff: pending
```

---

## 6. Definition of done

- [ ] Identity in the plan has a `floor_pct` term. 3.0% gate not moved.
- [ ] Standing rule written in tasks.md and pitfalls.md.
- [ ] S-05 marked conditional on true floor.
- [ ] F-A glow timed at 1 fps, both CRTCs, one exit summary.
- [ ] F-B clear timed at 1 fps, both CRTCs, live SHM surfaces.
- [ ] F-C collector/nvidia-smi counted or timed over the same M-1 window.
- [ ] F-D remainder is a number, not a name.
- [ ] Decision field set from the table in §3.4.
- [ ] 5.9 only if that decision is `5.9_UPDATE_MS`.
- [ ] User config.md5 unchanged.
- [ ] C-01 honored.
- [ ] AC5 still `pending`. Not forged.
- [ ] No Phase 3/4/6/7 source. No atlas. No Pulse. No deploy.
- [ ] No 605 ms figure used as an input.

Stop at the 5.8 (and 5.9 if gated-in) receipt. Bring it back. Do not declare S-04 met by rounding 3.0166 down.
```

# AGENT PROMPT — Round 10: S-04 AT_GATE, amend S-05, F8, Phase 8 Medium/Extreme
# Campaign: 20260903-matrixoverlay-render-remediation
# Against: 95e7bac   Branch: refactor/matrixoverlay.v2
# From: Grok Round 10 adjudication of PHASE_5.8_RECEIPT.md

```yaml
round: 10
against: 95e7bac
s04: AT_GATE                  # NOT met, NOT unmet-by-rounding
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
s04_spread_pp: 0.020
gate_numeral: 3.0             # do not retune to 3.05
choice: C_EXCEPTION
phase_6_1: SEQUEL_NOT_THIS_PASS
panel_cache: NOT_THIS_PASS
phase_5_9: NOT_THIS_PASS
phase_7: SHUT                 # until S-05 amendment is in the plan AND AC5
phase_8: OPEN                 # Medium + Extreme only; Minimal blocked on Pulse
f8: THIS_PASS                 # one-liner
do_not: [round_3.0166_down, move_gate, damage_tracking, panel_cache, atlas, pulse_impl, deploy, forge_AC5]
```

---

## 0. Rulings — bind these

```yaml
C:
  why: >
    60.7% → 3.00% is a 20× reduction. concept.md §III named "<1-3%", and 3 is
    the top of that range. The campaign tightened it to a point gate before
    any live term existed. Three 300 s windows sit on that point with 0.020 pp
    of instrument noise. Spending Phase 6 architecture on 0.17–0.37% against
    a 0.02 pp noise band is Context Erosion (§2.7).
  what_to_write: >
    S-04 status = S04_AT_GATE. Publish the three-window series and the mean.
    Do not cite 2.9966 as "the" result. The numeral 3.0 stays. The INTENT is
    satisfied. The point-gate is not.

A_why_not_now: >
    Damage tracking does not skip clear in fall mode: rain dirties the full
    4K+1080 every tick, so the opaque paint still runs. Phase 6.1 is a
    Pulse/static lever (R-04, R-12, new presentation module — shm.rs is at
    cap). Record as sequel for Pulse, not as an S-04 closer.

B_why_not_now: >
    Panel cache ceiling is glow = 0.1676% — smaller than the 0.020 pp spread.
    Does not get S-05 under 0.5% either (floor stays 0.5368%).

S-05_amendment:
  old: Pulse Mode M-1 < 0.5%
  why_impossible: true_floor 0.5368 >= 0.5; Pulse drawing nothing still fails
  new: >
    S-05a  Pulse rain+glow render term < 0.15% of one core
           (Pulse must be cheaper than fall's 1.78% rain+glow).
    S-05b  Whole-process M-1 at Pulse ≤ S-04 mean + 0.3 pp
           i.e. ≤ 3.30% given current floor. The 0.5% whole-process
           bound is retired; it assumed a floor that does not exist.
  phase_7: remains SHUT until this text is in the plan AND AC5 is signed.

AC5: pending HITL. 1 fps rain has not been seen by the user. Do not forge.

F8: delete or gate-off the unconditional
    config.cosmetics.rain_mode = "fall".to_string();
    after Config::load() in src/core/main.rs. One line. Blocking for
    Phase 8 Minimal (deferred) and for any later Pulse measurement.
    Record why the line existed if git log -S finds it; if not, say so.

Phase_8_scope:
  Medium:  target_fps=1, realism=4, glow_passes=3, rain_mode=fall
           THIS is the S04_AT_GATE configuration. Label it that way.
           Do not claim Medium meets "< 3%". Claim "3.00% ± 0.02 at 1 fps."
  Extreme: target_fps=30, realism=10, glow_passes=5, rain_mode=fall
           opt-in, exempt from S-04. Label it as exceeding the ambience budget.
  Minimal: DEFERRED. It requires Pulse (S-05 amended, Phase 7) and F8.
           Buttons may exist but must not write rain_mode=pulse until
           Phase 7 exists, OR they write it and F8 no longer clobbers —
           still no Pulse renderer, so the mode would draw nothing
           (pipeline.rs only draws rain when rain_mode=="fall").
           Do not ship a Minimal that blanks the screen.

update_ms_2s_to_5s:
  optional, user-ask only
  grounds: CLAUDE.md dGPU wakeup, NOT S-04
  nvidia-smi CPU is a child; M-1 will not move
```

Live terms — do not invent new ones.

```yaml
at_1_fps:
  series_m1: [3.0166, 2.9966, 2.9966]
  mean: 3.0033
  rain_pct: 1.6132
  present_pct: 0.3099
  clear_pct: 0.3690
  glow_pct: 0.1676
  render_pct: 2.4598
  floor_pct: 0.5368
```

---

## 1. Standing rules

- Method M-1 only. Publish series, not a chosen window.
- Launch: `cargo build --release || exit 1` then `./target/release/matrix-overlay & TESTPID=$!`
- Throwaway XDG_CONFIG_HOME for test runs. User config.md5 stays `4747e9c8a1bb239170f3a446d083a4e6`.
- C-01 175-line cap. C-02 `#[serde(default)]` on any new field.
- C-06 no deploy, no autostart, no kill of a user overlay.
- `/nodelete` [INTENT] untouchable.
- Standing rule already in pitfalls.md: ACs assert load behavior, not one step.
- F8 is THIS pass. Ungated `rain.update` remains recorded-only.

---

## 2. Commit 1 — documents only

`implementation-plan.md`, `tasks.md`, `docs/pitfalls.md`:

```
S-04 row:
  Status S04_AT_GATE
  Measurement: three 300 s M-1 windows at target_fps=1
               3.0166, 2.9966, 2.9966  mean 3.0033  spread 0.020
  Gate numeral 3.0 unchanged
  Exception paragraph: INTENT (60.7 → ~3, 20×) is met; the point-gate
  is inside instrument noise. Do not select the two low runs.

S-05 row: replace the 0.5% whole-process bound with S-05a / S-05b above.

§2.5:
  Phase 6.1 damage-tracking is a Pulse/static sequel. It does not skip
  clear in fall mode (full-surface dirt every tick). Not an S-04 closer.
  Panel cache deferred (0.17% < noise).

Phase 5 AC6: recorded UNMET-at-gate; not re-run to manufacture a pass.

Phase 7 entry: S-05 amendment (this commit) AND AC5 signed. Still shut.

Phase 8 entry: OPEN for Medium + Extreme. Minimal deferred.
```

Pitfalls stub: "S-04 sits at a pre-measurement point-gate; report the series."

---

## 3. Commit 2 — F8

```
src/core/main.rs
  Remove:
    config.cosmetics.rain_mode = "fall".to_string();
  or wrap behind an explicit debug flag that defaults OFF.
  After Config::load(), log the effective rain_mode once at info
  (one line, startup only — not per frame).
  AC: in-process value after load equals the JSON value.
      Verify with a unit-level fixture or a startup log assertion
      against a throwaway config that sets rain_mode to something
      other than "fall".
  git log -S 'rain_mode = "fall"' — record why it existed, or that
  no reason was found.
```

Without this, Phase 8 writing `rain_mode` is Ghost Logic.

---

## 4. Commit 3 — Phase 8 Medium / Extreme (not Minimal)

Existing tasks 8.1–8.7, narrowed:

```
8.1  Connect adv_w.0 / .1 / .2 to GuiEvent, same pattern as adv_w.4 → PurgeLogs.
     Minimal handler: either absent, or writes a no-op + log
     "Minimal requires Pulse Mode (Phase 7)". Do not blank the screen.

8.2  Preset table (binding):

     | Preset   | target_fps | realism | glow_passes | rain_mode | obligation          |
     | Medium   | 1          | 4       | 3           | fall      | S04_AT_GATE (3.00%) |
     | Extreme  | 30         | 10      | 5           | fall      | opt-in, exempt      |
     | Minimal  | —          | —       | —           | —         | deferred to Phase 7 |

     default_preset stays "medium".
     Extreme GUI label must say it exceeds the ambience budget.

8.3  Applying a preset writes the derived fields and persists via the
     existing .tmp-then-rename path.

8.4  Editing an individual field after a preset → perf_preset = "custom".

8.5  GUI reflects the active preset on open.

8.7  GL-2 sweep — wire or delete, no third option:
     - general.show_monitor_label
     - logging.build_logging_enabled
     - src/core/timer.rs (only caller of factory::create_collectors).
       Deleting timer.rs orphans factory.rs; decide both together.
     Verification: grep -rn each symbol; show a read, or show it gone.

Do not implement Pulse. Do not draw a Minimal path.
```

### Phase 8 ACs

```
AC1  Click Medium → config.json changes AND render is the current 1 fps fall
     (already running). Click Extreme → config.json changes AND fps moves
     toward 30 (use the Phase 1 fps instrument, ±10%, 30 s is enough).
AC2  Method M-1 is NOT required for Extreme. Medium is already measured
     (the S04_AT_GATE series). Do not spend another 3×300 s to re-prove 3.00%.
AC3  Individual edit → perf_preset "custom".
AC4  C-02: old config without perf_preset semantics still loads.
AC5  grep -rn perf_preset src/ shows a READ, not only a declaration.
```

Live Extreme run: throwaway XDG, 30–60 s, confirm fps ~30, then stop. Do not leave a 30 fps overlay running on the user's desktop.

---

## 5. Receipt schema

```yaml
phase: 8
git_sha: _
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
s05: AMENDED                    # S-05a / S-05b text landed
f8:
  line_removed: true
  git_log_S_reason: _           # or "none found"
  in_process_survives_restart: _
presets:
  medium: { writes_config: _, fps_unchanged_1: _ }
  extreme: { writes_config: _, wallclock_fps: _, metric_fps: _ }
  minimal: DEFERRED
gl2:
  show_monitor_label: wired | deleted
  build_logging_enabled: wired | deleted
  timer_rs: wired | deleted
  factory_rs: kept | deleted_with_timer
ac5_user_signoff: pending
phase_7: SHUT
phase_9: SHUT                   # needs AC5 + C-06
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"
```

---

## 6. Definition of done

- [ ] S-04 written as AT_GATE with the three-window series. Gate numeral 3.0 untouched. 3.0166 not rounded down.
- [ ] S-05a / S-05b in the plan. Old 0.5% whole-process bound retired with the reason.
- [ ] Phase 6.1 recorded as Pulse/static sequel; not started. Panel cache not started.
- [ ] F8 line gone or debug-gated default-off. In-process rain_mode survives load.
- [ ] Medium and Extreme buttons write config and change behavior. Minimal does not blank the screen.
- [ ] perf_preset is read. GL-2 each symbol wired or deleted.
- [ ] User config.md5 unchanged.
- [ ] C-01 honored.
- [ ] AC5 still pending. No deploy.
- [ ] No Phase 3/4/6/7 implementation. No atlas. No Pulse renderer.

Stop at the Phase 8 receipt. Do not open Phase 7. Do not open Phase 9.
```

.v2
# From: Grok Round 10 adjudication of PHASE_5.8_RECEIPT.md

```yaml
round: 10
against: 95e7bac
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
s04_spread_pp: 0.020
gate_numeral: 3.0
choice: C_EXCEPTION
phase_6_1: SEQUEL_NOT_THIS_PASS
panel_cache: NOT_THIS_PASS
phase_5_9: NOT_THIS_PASS
phase_7: SHUT
phase_8: OPEN
f8: THIS_PASS
do_not: [round_3.0166_down, move_gate, damage_tracking, panel_cache, atlas, pulse_impl, deploy, forge_AC5]
```

## 0. Rulings — bind these

```yaml
C:
  why: >
    60.7% → 3.00% is a 20× reduction. concept.md §III named "<1-3%", and 3 is
    the top of that range. The campaign tightened it to a point gate before
    any live term existed. Three 300 s windows sit on that point with 0.020 pp
    of instrument noise. Spending Phase 6 architecture on 0.17–0.37% against
    a 0.02 pp noise band is Context Erosion (§2.7).
  what_to_write: >
    S-04 status = S04_AT_GATE. Publish the three-window series and the mean.
    Do not cite 2.9966 as "the" result. The numeral 3.0 stays. The INTENT is
    satisfied. The point-gate is not.

A_why_not_now: >
    Damage tracking does not skip clear in fall mode: rain dirties the full
    4K+1080 every tick, so the opaque paint still runs. Phase 6.1 is a
    Pulse/static lever (R-04, R-12, new presentation module — shm.rs is at
    cap). Record as sequel for Pulse, not as an S-04 closer.

B_why_not_now: >
    Panel cache ceiling is glow = 0.1676% — smaller than the 0.020 pp spread.
    Does not get S-05 under 0.5% either (floor stays 0.5368%).

S-05_amendment:
  old: Pulse Mode M-1 < 0.5%
  why_impossible: true_floor 0.5368 >= 0.5; Pulse drawing nothing still fails
  new: >
    S-05a  Pulse rain+glow render term < 0.15% of one core
           (Pulse must be cheaper than fall's 1.78% rain+glow).
    S-05b  Whole-process M-1 at Pulse ≤ S-04 mean + 0.3 pp
           i.e. ≤ 3.30% given current floor. The 0.5% whole-process
           bound is retired; it assumed a floor that does not exist.
  phase_7: remains SHUT until this text is in the plan AND AC5 is signed.

AC5: pending HITL. 1 fps rain has not been seen by the user. Do not forge.

F8: delete or gate-off the unconditional
    config.cosmetics.rain_mode = "fall".to_string();
    after Config::load() in src/core/main.rs. One line.
    Record why it existed if git log -S finds it; if not, say so.

Phase_8_scope:
  Medium:  target_fps=1, realism=4, glow_passes=3, rain_mode=fall
           S04_AT_GATE config. Claim "3.00% ± 0.02 at 1 fps", not "< 3%".
  Extreme: target_fps=30, realism=10, glow_passes=5, rain_mode=fall
           opt-in, exempt. Label as exceeding the ambience budget.
  Minimal: DEFERRED. Requires Pulse. Do not blank the screen.

update_ms_2s_to_5s:
  optional, user-ask only
  grounds: CLAUDE.md dGPU wakeup, NOT S-04
```

```yaml
at_1_fps:
  series_m1: [3.0166, 2.9966, 2.9966]
  mean: 3.0033
  rain_pct: 1.6132
  present_pct: 0.3099
  clear_pct: 0.3690
  glow_pct: 0.1676
  render_pct: 2.4598
  floor_pct: 0.5368
```

## 1. Standing rules

- Method M-1 only. Publish series, not a chosen window.
- Launch: `cargo build --release || exit 1` then `./target/release/matrix-overlay & TESTPID=$!`
- Throwaway XDG_CONFIG_HOME. User config.md5 stays `4747e9c8a1bb239170f3a446d083a4e6`.
- C-01 175-line cap. C-02 `#[serde(default)]` on any new field.
- C-06 no deploy, no autostart, no kill of a user overlay.
- `/nodelete` [INTENT] untouchable.
- Standing rule already in pitfalls.md: ACs assert load behavior, not one step.
- F8 is THIS pass. Ungated `rain.update` remains recorded-only.

## 2. Commit 1 — documents only

`implementation-plan.md`, `tasks.md`, `docs/pitfalls.md`:

```
S-04 row:
  Status S04_AT_GATE
  Measurement: 3.0166, 2.9966, 2.9966  mean 3.0033  spread 0.020
  Gate numeral 3.0 unchanged
  Exception: INTENT (60.7 → ~3, 20×) is met; the point-gate is inside
  instrument noise. Do not select the two low runs.

S-05 row: replace the 0.5% whole-process bound with S-05a / S-05b.

§2.5:
  Phase 6.1 is a Pulse/static sequel. It does not skip clear in fall mode.
  Panel cache deferred (0.17% < noise).

Phase 5 AC6: recorded UNMET-at-gate; not re-run to manufacture a pass.
Phase 7 entry: S-05 amendment AND AC5 signed. Still shut.
Phase 8 entry: OPEN for Medium + Extreme. Minimal deferred.
```

## 3. Commit 2 — F8

```
src/core/main.rs
  Remove: config.cosmetics.rain_mode = "fall".to_string();
  or wrap behind a debug flag that defaults OFF.
  After Config::load(), log effective rain_mode once at info.
  AC: in-process value after load equals the JSON value.
      Throwaway config with rain_mode != "fall".
  git log -S 'rain_mode = "fall"' — record why, or none found.
```

## 4. Commit 3 — Phase 8 Medium / Extreme (not Minimal)

```
8.1  Connect adv_w.0 / .1 / .2 to GuiEvent, same pattern as adv_w.4 → PurgeLogs.
     Minimal handler: absent, or no-op + log "Minimal requires Pulse Mode (Phase 7)".
     Do not blank the screen.

8.2  Preset table:

     | Preset   | target_fps | realism | glow_passes | rain_mode | obligation          |
     | Medium   | 1          | 4       | 3           | fall      | S04_AT_GATE (3.00%) |
     | Extreme  | 30         | 10      | 5           | fall      | opt-in, exempt      |
     | Minimal  | —          | —       | —           | —         | deferred to Phase 7 |

     default_preset stays "medium".
     Extreme GUI label must say it exceeds the ambience budget.

8.3  Preset writes derived fields; persist via existing .tmp-then-rename.
8.4  Individual edit after preset → perf_preset = "custom".
8.5  GUI reflects the active preset on open.
8.7  GL-2 — wire or delete, no third option:
     - general.show_monitor_label
     - logging.build_logging_enabled
     - src/core/timer.rs (only caller of factory::create_collectors).
       Deleting timer.rs orphans factory.rs; decide both together.
     grep -rn each symbol; show a read, or show it gone.

Do not implement Pulse. Do not draw a Minimal path.
```

```
AC1  Medium → config changes, still 1 fps fall.
     Extreme → config changes, fps moves toward 30 (±10%, 30 s enough).
AC2  No new 3×300 s M-1 for Medium. Extreme not an S-04 gate.
AC3  Individual edit → perf_preset "custom".
AC4  C-02: old config still loads.
AC5  grep -rn perf_preset src/ shows a READ.
```

Live Extreme run: throwaway XDG, 30–60 s, confirm fps ~30, then stop. Do not leave a 30 fps overlay on the user's desktop.

## 5. Receipt schema

```yaml
phase: 8
git_sha: _
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
s05: AMENDED
f8:
  line_removed: true
  git_log_S_reason: _
  in_process_survives_restart: _
presets:
  medium: { writes_config: _, fps_unchanged_1: _ }
  extreme: { writes_config: _, wallclock_fps: _, metric_fps: _ }
  minimal: DEFERRED
gl2:
  show_monitor_label: wired | deleted
  build_logging_enabled: wired | deleted
  timer_rs: wired | deleted
  factory_rs: kept | deleted_with_timer
ac5_user_signoff: pending
phase_7: SHUT
phase_9: SHUT
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"
```

## 6. Definition of done

- [ ] S-04 written as AT_GATE with the three-window series. Gate numeral 3.0 untouched. 3.0166 not rounded down.
- [ ] S-05a / S-05b in the plan. Old 0.5% whole-process bound retired with the reason.
- [ ] Phase 6.1 recorded as Pulse/static sequel; not started. Panel cache not started.
- [ ] F8 line gone or debug-gated default-off. In-process rain_mode survives load.
- [ ] Medium and Extreme buttons write config and change behavior. Minimal does not blank the screen.
- [ ] perf_preset is read. GL-2 each symbol wired or deleted.
- [ ] User config.md5 unchanged.
- [ ] C-01 honored.
- [ ] AC5 still pending. No deploy.
- [ ] No Phase 3/4/6/7 implementation. No atlas. No Pulse renderer.

Stop at the Phase 8 receipt. Do not open Phase 7. Do not open Phase 9.

---

Same file:

---

```yaml
round: 11
against: be47456
ac5: pending_HITL
phase_7: SHUT
phase_9: SHUT
phase_10: OPEN
rain_update_gate: THIS_PASS
```

Copy from here.

---

# AGENT PROMPT — Round 11: Phase 10 docs + rain.update gate + AC5 protocol
# Against: be47456   Branch: refactor/matrixoverlay.v2
# Campaign: 20260903-matrixoverlay-render-remediation

```yaml
round: 11
against: be47456
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
ac5: pending_HITL
phase_7: SHUT
phase_9: SHUT
phase_10: OPEN
rain_update_gate: THIS_PASS
do_not: [pulse_impl, atlas, damage, panel_cache, install_sh, autostart, kill_overlay, forge_AC5, mutate_user_config]
```

## 0. Bindings

Phase 8 is complete. AC5 is the last human gate. Write a viewing protocol.
Do not set `ac5_user_signoff: accepted`.

Phase 7 and Phase 9 stay SHUT until the user writes that field.

`target_fps=2` is not free. At 1 fps, render = 2.460%. At 2 fps ≈ 4.92% render + 0.54% floor = **5.46% whole-process**, over the 3.0 gate. Cite both. Do not bump the default.

User config.md5 must stay `4747e9c8a1bb239170f3a446d083a4e6`.

## 1. Standing rules

- Method M-1 only. Publish the series. Do not cite 2.9966 as "the" result.
- Launch: `cargo build --release || exit 1` then `./target/release/matrix-overlay & TESTPID=$!`
- Throwaway XDG_CONFIG_HOME for any agent-owned run.
- C-01 175. C-02 serde default. C-06 no deploy.
- `/nodelete` [INTENT] untouchable.
- Standing rule: ACs assert load behavior (N events, rate), never one step.

## 2. Commit 1 — gate rain.update

`src/render/engine/pipeline.rs`: `rain.update` runs outside the `"fall"` gate.
Move it behind the same `rain_mode == "fall"` check that guards `rain.draw`.

- One if. No new module.
- Comment: Pulse leak; Phase 7 will own pulse physics if any.
- pipeline.rs is 131; stay ≤ 175.
- Existing tests still pass.
- Do not implement Pulse.

## 3. Commit 2 — Phase 10 documentation

### CLAUDE.md Rendering Pipeline
Step 1 says clear to **transparent**. Wrong. Code + pitfalls: `Operator::Source` + `rgba(0,0,0,1.0)` opaque black (CONFIRMED 2026-05-21). Correct the prose. Do not change the clear.

### CLAUDE.md module map
Flat `presentation.rs` is stale since e948079. Real path: `src/render/engine/presentation/{mod,shm,socket}.rs`. Add `src/core/config/presets.rs`, `src/core/telemetry/`. Do not invent `glyph_cache.rs`.

### CLAUDE.md Configuration
`target_fps` default 1, clamp 1..=60. Preset table: Medium / Extreme / Minimal-deferred. S04_AT_GATE series.

### pitfalls.md — Pango font-cache eviction
CONFIRMED-BUG style:
- Cycling N FontDescriptions through one layout evicts before reuse.
- Lab: MRC 605 ms / 74× control (LAB_F1). Live: 9.62 ms / 1.25× (F1 not live).
- Cost lands on first `show_layout` after size change, not on `set_font_description`.
- `s.depth` stays continuous. Phase 3 demoted.

### pitfalls.md — Mock Trap
`test_render_optimization_bench` measured one font size through one layout while production cycled ~162 sizes. Green for months, ~90× off. Rule: production code, production-shaped inputs, or label it a control.

Also record:
- `test_stability_no_flicker` guarded `update_ms` while the tick was 33 ms.
- `test_layout_predictability` shipped with every assert commented out.
- `tests/window_integration.rs` asserts 1920×1080; RandR is 4096×2160 + 1920×1080 (R-11).
- `tests/metrics_tests.rs` never compiled (`NvidiaSmiCollector::new_with_command`) — MT-3.
- Standing rule (4th recurrence): assert behaviour under load, never a local copy of the production expression, never a property of one step.
- F8 origin: d2f61a1 "FORCE OVERRIDE: Ensure rain is enabled for verification". Companion `realism_scale=8` was cleaned; this line survived 7 months.
- C-02 vs GL-2 delete: a field in the user's live config.json cannot be removed from the struct without `deny_unknown_fields` rejecting the file.
- `timer.rs` was a second dead metrics loop carrying the F4 1 ms fail-open. No callers. Deleted with `factory.rs`.

### Citation
`concept.md` §IV = 500 ms minimum update. "1Hz or 0.5Hz is sufficient" and "No flashing" live at pitfalls.md:70-72. Do not re-attribute.

### DevJournal.md
2026-09-03 → 2026-09-04: 60.7% discovered, F2 16× lie, A-01 1.3 vs live 30.2, MRC 61× vs live, governor, identity close, S04_AT_GATE 3.0033 ± 0.020, F8 origin d2f61a1, timer.rs was a second F4. Do not write "S-04 passed".

## 4. AC5 protocol — write it; do not execute a verdict

May start a throwaway-XDG release binary at Medium (`target_fps=1`), then leave it running. Do not mark accepted or rejected. Do not kill a user overlay. Do not touch `~/.config/matrix-overlay/config.json`.

```
cargo build --release || exit 1
XDG_CONFIG_HOME=/tmp/mo-ac5 HOME=/tmp/mo-ac5-home \
  ./target/release/matrix-overlay
# Medium defaults: 1 fps fall, realism 4
```

Look ~30 s on BOTH CRTCs (HDMI-1-0 4K and eDP 1080p):
- Slow pulse of the desktop, or slideshow / stall?
- Any strobing (C-05 / "No flashing")?
- Metrics panel readable?

User replies with exactly one of:
```
ac5_user_signoff: accepted
ac5_user_signoff: rejected_want_2fps
ac5_user_signoff: rejected_other <reason>
```

If `rejected_want_2fps`: whole-process at 2 fps ≈ **5.46%**, over the 3.0 gate. New conversation, not a silent bump.

If the user is not present, launch nothing. Leave `user_verdict: pending`.

## 5. Receipt

```yaml
phase: 10
git_sha: _
rain_update_gated: true
claude_md:
  clear_corrected: _
  presentation_path_corrected: _
  target_fps_documented: _
pitfalls:
  font_cache_entry: _
  mock_trap_entry: _
  standing_rule_present: true
  f8_origin_d2f61a1: _
devjournal: _
ac5_protocol_written: true
ac5_user_signoff: pending
phase_7: SHUT
phase_9: SHUT
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"
```

## 6. Definition of done

- [ ] `rain.update` only runs when `rain_mode == "fall"`.
- [ ] CLAUDE.md clear + presentation path + target_fps/presets documented.
- [ ] pitfalls.md has font-cache, Mock Trap, standing rule, R-11/MT-3, F8 origin.
- [ ] DevJournal records the session numbers. Does not say "S-04 passed".
- [ ] AC5 protocol in the receipt. Field still `pending` unless the user wrote a verdict.
- [ ] User config.md5 unchanged. C-01 honored.
- [ ] No Pulse renderer. No atlas. No install.sh. No Phase 7/9.

Stop. Wait for the user's AC5 line. Phase 9 is C-06, not a courtesy install.

---

Same file:

---

# AGENT PROMPT — Round 11: Phase 10 docs + rain.update gate + AC5 protocol
# Against: be47456   Branch: refactor/matrixoverlay.v2
# Campaign: 20260903-matrixoverlay-render-remediation
# From: Grok Round 11 adjudication of PHASE_8_RECEIPT.md

```yaml
round: 11
against: be47456
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
s04_mean: 3.0033
ac5: pending_HITL
phase_7: SHUT
phase_9: SHUT
phase_10: OPEN
rain_update_gate: THIS_PASS
do_not: [pulse_impl, atlas, damage, panel_cache, install_sh, autostart, kill_overlay, forge_AC5, mutate_user_config]
```

## 0. Bindings

Phase 8 is complete. AC5 is the last human gate. You will write a viewing protocol.
You will not set `ac5_user_signoff: accepted`.

Phase 7 (Pulse) and Phase 9 (deploy) stay SHUT until the user writes that field.

`target_fps=2` is not a free option. Live terms at 1 fps:
  rain 1.613 + present 0.310 + clear 0.369 + glow 0.168 = 2.460
  at 2 fps ≈ 4.92% render + floor 0.54% = **5.46% whole-process**, over the 3.0 gate.
Cite both numbers if asked. Do not bump the default.

User config.md5 must stay `4747e9c8a1bb239170f3a446d083a4e6`.

## 1. Standing rules

- Method M-1 only. Publish the series. Do not cite 2.9966 as "the" result.
- Launch: `cargo build --release || exit 1` then `./target/release/matrix-overlay & TESTPID=$!`
- Throwaway XDG_CONFIG_HOME for any agent-owned run.
- C-01 175. C-02 serde default. C-06 no deploy.
- `/nodelete` [INTENT] untouchable.
- Standing rule: ACs assert load behavior (N events, rate), never one step.

## 2. Commit 1 — gate rain.update

`src/render/engine/pipeline.rs`: `rain.update` runs outside the `"fall"` gate.
Move the update behind the same `rain_mode == "fall"` check that guards `rain.draw`.

- One if. No new module.
- Comment: Pulse leak; Phase 7 will own pulse physics if any.
- C-01: pipeline.rs is 131; stay ≤ 175.
- Existing asd/governor/lib tests still pass.
- Do not implement Pulse.

## 3. Commit 2 — Phase 10 documentation

Docs only, plus the comment that rides (2).

### CLAUDE.md Rendering Pipeline
- Step 1 currently says clear to **transparent**. Wrong.
- Code + pitfalls: `Operator::Source` + `rgba(0,0,0,1.0)` opaque black
  (CONFIRMED 2026-05-21). Correct the prose. Do not change the clear.

### CLAUDE.md module map
- Flat `presentation.rs` is stale since e948079.
- Real path: `src/render/engine/presentation/{mod,shm,socket}.rs`.
- Add `src/core/config/presets.rs`, `src/core/telemetry/`.
- Do not invent `glyph_cache.rs` — Phases 3–4 never ran.

### CLAUDE.md Configuration
- `target_fps` default 1, clamp 1..=60.
- Preset table: Medium / Extreme / Minimal-deferred.
- S04_AT_GATE series.

### pitfalls.md — Pango font-cache eviction
CONFIRMED-BUG style, matching SHM / OverrideRedirect entries:
- Mechanism: cycling N FontDescriptions through one layout evicts before reuse.
- Lab: cargo-test MRC 605 ms / 74× its control (LAB_F1).
- Live: 9.62 ms / 1.25× in-process control (F1 not live).
- Cost lands on first `show_layout` after size change, not on `set_font_description`.
- `s.depth` stays continuous; only font size may be quantized. Phase 3 demoted.

### pitfalls.md — Mock Trap
`test_render_optimization_bench` measured one font size through one layout
while production cycled ~162 sizes. Green for months, ~90× off.
Rule: production code, production-shaped inputs, or label it a control.

Also record:
- `test_stability_no_flicker` guarded `update_ms` while the tick was 33 ms.
- `test_layout_predictability` shipped with every assert commented out.
- `tests/window_integration.rs` maps real desktop windows and asserts
  1920×1080; RandR here is 4096×2160 + 1920×1080 (R-11).
- `tests/metrics_tests.rs` never compiled
  (`NvidiaSmiCollector::new_with_command`) — MT-3.
- Standing rule (4th recurrence): assert behaviour under load
  (N events, achieved rate), never a local copy of the production
  expression and never a property of one step.
- F8 origin: d2f61a1 "FORCE OVERRIDE: Ensure rain is enabled for verification".
  Companion `realism_scale=8` was cleaned; this line survived 7 months.
- C-02 vs GL-2 delete: a field in the user's live config.json cannot be
  removed from the struct without `deny_unknown_fields` rejecting the file.
- `timer.rs` was a second dead metrics loop carrying the F4 1 ms fail-open.
  No callers. Deleted with `factory.rs`.

### Citation
`concept.md` §IV = 500 ms minimum update. "1Hz or 0.5Hz is sufficient"
and "No flashing" live at pitfalls.md:70-72. Do not re-attribute.

### DevJournal.md
2026-09-03 → 2026-09-04 session: 60.7% discovered, F2 16× lie,
A-01 1.3 vs live 30.2, MRC 61× vs live, governor, identity close,
S04_AT_GATE 3.0033 ± 0.020, F8 origin d2f61a1, GL-2 timer.rs was a second F4.
Do not write "S-04 passed".

## 4. AC5 protocol — write it; do not execute a verdict

Agent may start a throwaway-XDG release binary at Medium (`target_fps=1`)
so there is something to look at, then leave it running. Do not mark
accepted or rejected. Do not kill a user overlay. Do not touch
`~/.config/matrix-overlay/config.json`.

```
cargo build --release || exit 1
XDG_CONFIG_HOME=/tmp/mo-ac5 HOME=/tmp/mo-ac5-home \
  ./target/release/matrix-overlay
# Medium defaults: 1 fps fall, realism 4
```

Look for ~30 s on BOTH CRTCs (HDMI-1-0 4K and eDP 1080p):
- Does the rain read as a slow pulse of the desktop, or as a slideshow / stall?
- Any strobing (C-05 / pitfalls "No flashing")?
- Metrics panel readable?

User replies with exactly one of:
```
ac5_user_signoff: accepted
ac5_user_signoff: rejected_want_2fps
ac5_user_signoff: rejected_other <reason>
```

If `rejected_want_2fps`: whole-process at 2 fps ≈ **5.46%**, over the 3.0 gate.
That is a new conversation, not a silent bump.

If the user is not present, launch nothing. Leave `user_verdict: pending`.

## 5. Receipt

```yaml
phase: 10
git_sha: _
rain_update_gated: true
claude_md:
  clear_corrected: _
  presentation_path_corrected: _
  target_fps_documented: _
pitfalls:
  font_cache_entry: _
  mock_trap_entry: _
  standing_rule_present: true
  f8_origin_d2f61a1: _
devjournal: _
ac5_protocol_written: true
ac5_user_signoff: pending
phase_7: SHUT
phase_9: SHUT
s04: AT_GATE
s04_series: [3.0166, 2.9966, 2.9966]
config_json_md5: "4747e9c8a1bb239170f3a446d083a4e6"
```

## 6. Definition of done

- [ ] `rain.update` only runs when `rain_mode == "fall"`.
- [ ] CLAUDE.md clear + presentation path + target_fps/presets documented.
- [ ] pitfalls.md has font-cache, Mock Trap, standing rule, R-11/MT-3, F8 origin.
- [ ] DevJournal records the session numbers (series, live 30.2, MRC 605, F8 d2f61a1).
- [ ] AC5 protocol in the receipt. Field still `pending` unless the user wrote a verdict.
- [ ] User config.md5 unchanged. C-01 honored.
- [ ] No Pulse renderer. No atlas. No install.sh. No Phase 7/9.

Stop. Wait for the user's AC5 line. Phase 9 is C-06, not a courtesy install.
```