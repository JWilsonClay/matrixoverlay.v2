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
