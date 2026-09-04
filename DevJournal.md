2026-05-20 Antigravity broke because Google retired Gemini 3 Flash and the token limits are fractioned. I'm back to manual coding with Grok.  Not hating it. I like Grok, it's just much slower.  But good use of my paid subscription!

---

## 2026-05-21 -- LG TV 4K Crash Fix, Presenter Refactor, SHM Zero-Copy, SHM Sync

### Session Summary
This session with Claude resolved a crash on the LG TV (HDMI 4K, 4096×2160), refactored the rendering presentation layer, introduced MIT-SHM zero-copy frame delivery, and fixed a subsequent SHM race condition that caused visual flashing.

### Problem 1: Hard Crash on 4K Display
**Root Cause**: A single `xcb::x::PutImage` call sending ~31 MB of pixel data per frame exceeded the X11 server's maximum request size limit. The request silently failed or caused a connection error, crashing the overlay.

**Resolution (Phase 1)**: Modified `src/render/engine/presentation.rs` to stripe `PutImage` calls. Each stripe is sized to fit within `conn.get_maximum_request_length() * 4 - 28` bytes (28 bytes = 7-word PutImage wire header). Key corrected formula details:
- Use `surface.stride()` — not `width * 4` — because Cairo may pad rows for alignment.
- Use `.max(1)` guard on stripe height to prevent division-by-zero.
- Call `stride()`, `height()`, `width()` before `data()` — Cairo's `data()` takes a mutable borrow.

Commit: `ea2cbd1`

### Problem 2: Pre-existing Visual Regressions (from prior AI session)
Two regressions were introduced by a previous Antigravity session:

1. **`OverrideRedirect(true)` in `creation.rs`** — bypasses the window manager entirely. EWMH atoms (`_NET_WM_WINDOW_TYPE_DESKTOP`, `_NET_WM_STATE_BELOW`) are silently ignored, so the window stacks above everything instead of behind it. Fixed: reverted to `OverrideRedirect(false)`.

2. **`Operator::Clear` in `pipeline.rs::clear()`** — makes the background fully transparent instead of opaque black. The prior fix for the crash had also removed the black background. Fixed: restored `Operator::Source + set_source_rgba(0,0,0,1)`.

Commit: `ea2cbd1`

### Phase 2: Presenter Trait Refactor
Extracted the raw presentation logic into a `Presenter` trait (`src/render/engine/presentation/mod.rs`) with two implementations:
- `SocketPresenter` — the working stripe loop; baseline safe path.
- `ShmPresenter` — zero-copy SHM path (Phase 3).

`Renderer` now holds `Box<dyn Presenter>` instead of a raw `ImageSurface`. `pipeline.rs` calls `self.presenter.surface()` and `self.presenter.present()`. No behavior change from Phase 1.

Commit: `e948079`

### Phase 3: MIT-SHM Zero-Copy Presentation
Replaced socket-based `PutImage` (which copies ~34 MB over the X socket per frame at 4K) with `xcb::shm::PutImage`, which reads pixel data directly from a shared memory segment.

Key implementation details:
- `shmget(IPC_PRIVATE)` → `shmat` → `IPC_RMID` (auto-destroy on detach for crash safety).
- `ShmBuffer` unsafe wrapper: satisfies `AsMut<[u8]> + Send` for `ImageSurface::create_for_data` without triggering allocator free on drop.
- `conn: Arc<xcb::Connection>` stored in `ShmPresenter` so `Drop` can call `xcb::shm::Detach`.
- **Drop ordering is critical**: `surface.take()` (drop Cairo first) → `shm::Detach` → `libc::shmdt`. Calling `shmdt` while Cairo still holds a pointer to the SHM region is undefined behavior.
- Feature gate: `xcb::shm::QueryVersion {}` probe at startup; falls back to `SocketPresenter` if SHM is unavailable.
- `xcb::shm::PutImage::format` is `u8`, value `2` (ZPixmap) — not the `x::ImageFormat` enum.

Startup log confirmed: `[Presenter] MIT-SHM available — using zero-copy ShmPresenter` on both monitors (laptop 1080p + LG TV 4K).

Commit: `8a02ba5`

### Problem 3: SHM Race Condition (Flashing / Rain Stutter)
After Phase 3 was live, the overlay showed rapid on/off flashing of metrics and rain stutter.

**Root Cause**: `ImageSurface::create_for_data` causes Cairo to write *directly* into the SHM region (no copy). When the 30fps tick fired the next `clear()` call, it overwrote the SHM buffer with black while the X server was still reading the previous `ShmPutImage` from that same memory. The X server caught mid-cleared or mid-drawn frames — hence the flash-to-black and partial frames.

**Resolution (Phase 4)**: Added a `pre_draw()` hook to the `Presenter` trait, called at the top of `pipeline.rs::draw()` before `CairoContext::new`. `ShmPresenter::pre_draw()` issues a `GetInputFocus` round-trip request and waits for the reply. By the time the X server replies, it has sequentially processed all prior requests including the `ShmPutImage`, so the SHM region is safe to overwrite. `SocketPresenter::pre_draw()` is a no-op (socket PutImage copies data before returning). Round-trip overhead is <1ms on a local X socket.

Commit: `380107f`

### Architecture State After This Session

**Rendering pipeline** (`src/render/engine/`):
```
pipeline.rs::draw()
  └─ presenter.pre_draw()   ← SHM sync gate (new)
  └─ CairoContext::new(presenter.surface())
  └─ clear() + rain + metrics
  └─ presenter.present()    ← SHM: PutImage zero-copy; Socket: stripe loop
```

**Presenter selection** (`presentation/mod.rs::create_presenter()`):
```
QueryVersion → SHM available?
  Yes → ShmPresenter::new() → fallback to SocketPresenter on init error
  No  → SocketPresenter
```

**Presenter trait**:
```rust
fn pre_draw(&mut self, conn: &xcb::Connection) -> Result<()>;  // sync gate
fn surface(&self) -> &ImageSurface;
fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()>;
fn resize(&mut self, w: u16, h: u16) -> Result<()>;
```

### Hardware Confirmed Working
- Dell G15 laptop display: 1920×1080, ShmPresenter
- LG TV via HDMI: 4096×2160, ShmPresenter
- Overlay CPU: ~4% (acceptable for 4K zero-copy at 30fps)
- All metrics rendering correctly, black background, correct desktop layering

rebuild and retest command:
cargo build --release
RUST_LOG=debug ./target/release/matrix-overlay

# Matrix Overlay v2 Development Journal

## 2026-05-15 -- Workspace Audit & Gap Analysis

### Today's Progress
- Completed a comprehensive forensic investigation into the workspace intent vs. implementation.
- Performed a substrate parity audit, identifying several "Ghost Logic" items where documentation and code have diverged.
- Initialized the Governance Substrate documentation and established the Matrix Overlay soul and purpose report.

### Architecture Updates
- Identified a structural conflict in the X11 window management: The current "click-through" implementation (using XShape empty input regions) prevents the capture of hover/click events on metrics.
- Confirmed that metrics collection is correctly decoupled into a dedicated timer thread, maintaining the <1% CPU usage target.

### Implementation Plan
- **Stage 2a: Interaction Bridge**: Reconcile click-through stability with the need for interactive notification boxes (potential use of secondary overlay windows or dynamic shape switching).
- **Stage 2b: Tray Notifications**: Implement a signaling mechanism between the Metrics thread and the Tray UI to display desktop bubbles for Git auto-commits and AI insights.
- **Stage 2c: Secure Auto-updater**: Develop a GitHub-integrated update client that fetches minified/stripped binaries for a "closed-source" simulation.

### Key Decisions & Learnings
- **Decision**: Prioritize the "Pulse" mode for ASD-friendly environments as the primary alternative to the high-motion Matrix rain.
- **Learning**: Hybrid graphics systems (AMD iGPU + NVIDIA dGPU) require distinct metric collectors (`hwmon` vs `nvidia-smi`) which are now correctly differentiated in the codebase.

### Challenges & Resolutions
- **Challenge**: Intent for "Drag & Drop" repositioning was found missing in the code (currently uses GTK Up/Down buttons). 
- **Resolution**: Deferred drag-and-drop to a future UI refactor; existing buttons satisfy the "customizable order" requirement for now.

### Next Actions
1. Design the GitHub Update Protocol (Item 5).
2. Implement Tray Update Notifications (Item 4).
3. Hardened binary production (Minification/Symbol Stripping).

---
**Documentation Changes Summary**
- Created: `DevJournal.md`
- Updated: `MANIFEST.md` (via Sentinel scan)
- **[INJECTION - 2026-05-15]**: Migrated all legacy infrastructure, diagnostic scripts, and workflow protocols into this journal.

---

## Historical Infrastructure Recovery & Workflow Legacy Migration
**[INJECTION - 2026-05-15] -- Recovered from legacy docs (01/02/03 Role Starters, pipeline.md, codebase.md)**

### I. The 8-Stage Execution Framework
This framework represents the project's foundational development lifecycle. Each stage is designed to be executed by an AI Agent with specific role prompts.

| Stage | Focus Area | Critical Directive |
| :--- | :--- | :--- |
| **S1** | **Structure & Style** | Enforce Rust idioms; ensure modularity for new features. |
| **S2** | **Functional Correctness** | Verify behavior matches intent; identify logic gaps. |
| **S3** | **Debugging** | Diagnose/fix bugs; log reproduction steps and fix logic. |
| **S4** | **Security Audit** | Threat model input validation, data storage, and auth. |
| **S5** | **Testing Strategy** | Implement Unit, Integration, and Edge Case test suites. |
| **S6** | **Optimization** | Identify bottlenecks; minimize resource/memory pressure. |
| **S7** | **Documentation** | Update READMEs, docstrings, and maintainability notes. |
| **S8** | **Integration Review** | Final architectural check across all modules. |

### II. Quick Iteration Protocol
A high-velocity protocol for rapid development on a single file:
- **Identifier**: `### QUICK ITERATION - Stage #[N]`
- **Fields**: `Current File/Module`, `Specific Focus`, `Code Snippet`.
- **Intent**: Maximize token efficiency by omitting full project context during surgical edits.

### III. Python Diagnostic & Verification Script
A standalone verification engine recovered from the `codebase.md` manifest.
- **Structure Fixer**: Moves `.rs` files from root to `src/` to maintain substrate hygiene.
- **Build Auditor**: Automated execution of `cargo build --release` and `cargo test --all-targets`.
- **Runtime Gate**: Runs binary for 5s with `RUST_LOG=info` to verify startup stability; kills via timeout.

### IV. Role-Based Governance & Dissent Protocol
The development triad ensures high-fidelity execution through distributed authority:
- **Architect**: Technical vision and Stage 0 planning.
- **Project Manager**: Transmutes stages into numbered implementation directives.
- **Systems Engineer**: Implementation specialist with a **Mandatory Dissent Right** to challenge unoptimized or illogical directives.

### V. Persona & Environmental Constraints
- **Hardware Target**: Dell G15 Ryzen (Linux/Pop!_OS).
- **User Skills**: Basic Rust proficiency with a strong focus on AI-assisted development growth.
- **Success Metrics**: 100% "Instant Activation" for all togglable features; <1% CPU impact in background modes.

---

## VI. Full Legacy Stage Templates (S0-S8)
**[INJECTION - 2026-05-15] -- Full forensic recovery of AI prompting blueprints from pipeline.md**

### Stage 0: Requirements & Planning
- **Goal**: Capture requirements, skill gaps, tech stack, and success metrics.
- **Template Fields**:
    - `What I Want to Build`: (e.g., Matrix Overlay v2 with Git tracking).
    - `Skills Have/Need`: (e.g., Have Rust; Need git2/Ollama).
    - `Constraints`: (Time, Platform: Pop!_OS, Budget: $0).
    - `Non-functional Requirements`: (Performance: <1%, Security: Local-only).
    - `Success Metrics`: (No crashes, accurate delta tracking).

### Stage 1: Code Structure & Style
- **Goal**: Ensure readability, modularity, and Rust idiom compliance.
- **Checklist**:
    - [ ] Updated Global Project Context?
    - [ ] Saved refactored code?
    - [ ] Updated Last Major Change?

### Stage 2: Functional Correctness
- **Goal**: Verify code matches behavior and requirements.
- **Template Fields**: `Intended Behavior`, `Key Modules to Verify`, `Code to Analyze`.

### Stage 3: Debugging / Specific Issues
- **Goal**: Triage and fix targeted bugs.
- **Template Fields**: `Error Message`, `Reproduction Steps`, `Expected vs Actual Behavior`.

### Stage 4: Security Audit
- **Goal**: Threat modeling and vulnerability identification.
- **Focus**: Input validation, data storage, credential handling.

### Stage 5: Testing Strategy
- **Goal**: Implement Unit, Integration, and Edge Case tests.
- **Focus**: `Ollama output mocks`, `Git2 failure modes`, `Multi-monitor XCB edge cases`.

### Stage 6: Performance / Optimization
- **Goal**: Identify bottlenecks and improve resource efficiency.
- **Focus**: `CPU throttling via sysinfo`, `Benchmark Ollama calls`, `Cairo dirty rects`.

### Stage 7: Documentation & Maintainability
- **Goal**: Technical documentation and README updates.
- **Checklist**: `Comments`, `Docstrings`, `README current`.

### Stage 8: Final Integration Review
- **Goal**: Ensure cross-module consistency.
- **Focus**: `Integration points`, `Dependencies`, `Final readiness`.

---

## VII. Theoretical Pipeline Blueprint (S1-S8)
**[INJECTION - 2026-05-15] -- Forensic Mapping of Master Inventory to Execution Stages**

This blueprint provides the "Governance Map" for the Matrix Overlay codebase. It transforms the flat file inventory into a multidimensional maintenance framework.

### Stage 1: Code Structure & Style (The Skeleton)
*   **Substrates**: `src/lib.rs`, `src/main.rs`, `src/core/mod.rs`, `src/metrics/mod.rs`, `src/ui/mod.rs`.
*   **Role**: Enforces the 125-line limit and Sovereign Substrate boundaries.

### Stage 2: Functional Correctness (The Logic)
*   **Substrates**: All `collectors/*.rs`, `manager.rs`, `physics.rs`, `ai.rs`.
*   **Role**: Logic-gate verification for data collection and Matrix rain simulation.

### Stage 3: Debugging / Specific Issues (The Triage)
*   **Substrates**: `build_logger.rs`, `logging/mod.rs`, `reproduce_tray.rs`.
*   **Role**: Forensic visibility into runtime anomalies and substrate trauma.

### Stage 4: Security Audit (The Shield)
*   **Substrates**: `validation.rs`, `shape.rs`, `storage.rs`, `init.rs`.
*   **Role**: Regex-constrained sanitization and defense against path traversal/injection.

### Stage 5: Testing Strategy (The Assurance)
*   **Substrates**: `/tests/` directory and internal mocking interfaces (`ai.rs`, `weather.rs`).
*   **Role**: Stability verification through failure-mode simulation and E2E gates.

### Stage 6: Performance / Optimization (The Pulse)
*   **Substrates**: `threads/mod.rs`, `dispatch.rs`, `update.rs`.
*   **Role**: Protecting the <1% CPU target through async delivery and redraw throttling.

### Stage 7: Documentation & Maintainability (The Legacy)
*   **Substrates**: `defaults.rs`, `concept.md`, `DevJournal.md`.
*   **Role**: Intent preservation and copious detail injection for future auditability.

### Stage 8: Final Integration Review (The Whole)
*   **Substrates**: `gui/mod.rs`, `tray.rs`, `creation.rs`, `atoms.rs`.
*   **Role**: Orchestration of decoupled domains into a singular operating HUD.

---

## VIII. SEITS Initialization & Governance Pivot
**[INJECTION - 2026-05-15] -- Transitioning to High-Fidelity Validation**

This entry anchors the pivot from Phase 0 (Consolidation) to Phase 5 (Verification).

### 1. AI Substrate Transition
- **From**: Ollama (Local API)
- **To**: LiteRT + Gemma 2 (4b)
- **Reason**: Precise regex-constrained inference and direct local control via the `sovereign-litert` substrate at `/home/jwils/Public/sovereign-litert`.

### 2. Tiered Line-Limit Governance
- **Skeleton Limit (125 Lines)**: Maintained for initial decomposition and modular spikes.
- **Hardened Limit (175 Lines)**: Established for mature modules to accommodate "Copious Detail" (docstrings, extensive validation, and forensic logging) without forced, unnatural fragmentation.

### 3. SEITS Orchestrator (v1.0)
- **Design**: 5-Phase iterative validation loop.
- **Protocols**: Mandatory **Intelligence Bridge Declaration** and **HOT Execution** mandates per the `/iterate-test` protocol.
- **Output**: `VALIDATION_RECEIPT.md`.

---

## 2026-09-03 → 2026-09-04 — Render Substrate Remediation Campaign

**Intent (`/nodelete`, unchanged all campaign):** *Make the Matrix Overlay stop eating 60% of a CPU
core, and make it honest about what it costs.*

**Result: 60.7% → 3.0033% of one core — a 20× reduction — with every term in that 3.00% named and
measured.** S-04's point gate of 3.0% is **not** declared passed; see `S04_AT_GATE` below.

### What the instruments were lying about

- **F2 — a 16× under-report.** `overlay_cpu` divided `sysinfo`'s process CPU by the core count, so
  the overlay displayed **3.79%** while Method M-1 measured **59.54%**. The overlay was wrong about
  its own cost by more than an order of magnitude, in a metric it drew on screen. Fixed and verified
  at **−0.84 pp** against M-1, where the old code would have shown **−40.23 pp**.
- **A-01 — the frame rate was inferred, not measured, and the inference was off by 23×.** The plan
  reasoned from a ~750 ms frame cost to ~1.3 fps. The live loop ran at **30.2 fps**, flat across 12
  minutes with no decay. Every frame-budget number derived from 750 ms was wrong by ~60×.
- **The Mock Trap.** `test_render_optimization_bench` had guarded the render path for months while
  measuring one font size through one layout — a path production never takes. ~90× off. Deleted.

### The measurement that reversed the campaign

The replacement MRC satisfied every anti-Mock-Trap rule that had been written down — production
`RainManager::draw`, production geometry, primed streams — and still disagreed with the running
process by **61×** (605 ms vs 9.62 ms). Glyph counts matched within 6%, so it was not volume, not the
clip guard, not priming. **Font-size churn costs the test process 74× its control and the overlay
process 1.25×.** F1 is real in the lab and absent in production. Phases 3–4 (bucketing, glyph atlas)
were **demoted** rather than built: they would have attacked a cost the live process does not pay.

### What actually fixed it

The frame governor. The 33 ms tick had a fail-open — a slow frame slept 1 ms and re-queued, so the
loop ran fastest exactly when it was already too slow. Replaced with a monotonic deadline and
`target_fps`, default **1**. Missed ticks are skipped, never queued.

### The identity, closed

At `target_fps = 1`, per tick across both CRTCs: rain 16.2128 ms · present 3.1148 · clear 3.7088 ·
glow 1.6848 → render **2.4598%**; plus a frame-rate-**independent** floor of **0.5368%** (metrics
collectors, GTK/tray, XCB). **2.4598 + 0.5368 = 2.9966 = M-1, exactly.**

The glow had been named as the leading suspect in three consecutive receipts. Measured, it is the
**smallest** term. `clear` is 2.2× larger — and the 1080p panel costs *more* glow than the 4K one,
because glow scales with metric count, not pixels.

### `S04_AT_GATE`

Three 300 s M-1 windows: **3.0166, 2.9966, 2.9966** — mean **3.0033**, spread **0.020 pp**, one of
three over the 3.0 gate. **This is not written down as "S-04 passed."** `concept.md` §III asks for
"< 1–3%" and the result sits on the top of that range, inside the instrument's own noise. The
`[INTENT]` is met; the point gate — which this campaign tightened from a range before a single live
term existed — is recorded as a written exception. The two remaining levers were priced (panel cache
0.1676%, damage tracking 0.3690%) and declined; damage tracking cannot skip `clear` while rain
dirties both full surfaces every tick.

### Two things that had been sitting in the tree

- **F8** — `main.rs` overwrote `rain_mode` with `"fall"` after every config load, making all other
  modes unreachable. Origin: `d2f61a1` (2026-02-28), commented *"FORCE OVERRIDE: Ensure rain is
  enabled for verification."* A debugging line that survived **seven months**. Its companion from the
  same commit was cleaned up; this one was not.
- **`timer.rs`** — a second, dead copy of the metrics loop carrying the *same* F4 fail-open, with no
  callers. `factory.rs` existed only to serve it. Both deleted. Had the F4 hunt started there, it
  would have looked like the fix site and fixing it would have changed nothing.

### The rule this campaign paid for four times

**A performance AC asserts behaviour under load — N events and the achieved rate — never a property
of one step, and never against a local copy of the production expression.** Phase 1 asserted against a
local copy. Phase 2 substituted `Config::default()`. Phase 5's governor test **passed** against a
deliberately fail-open implementation. Phase 8's preset script re-derived the preset table. Each was
caught by reinstating the defect; none by reading the test.

### Open

**AC5 — a human has not yet looked at 1 fps rain.** No measurement can decide whether it reads as
calm or as broken. Phases 7 (Pulse Mode) and 9 (deploy) stay shut behind it.
