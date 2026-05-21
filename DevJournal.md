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
