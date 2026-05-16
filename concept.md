# Matrix Overlay v2: Sovereign Intent & Concept

## I. Mission Statement
**Matrix Overlay v2** is a high-performance, low-resource desktop transparency substrate for Linux (Pop!_OS). It blends immersive **Matrix-themed aesthetics** with **forensic productivity tracking** to provide developers with a real-time pulse of their system and their output.

---

## II. Core Concept: The Sovereign HUD
The application functions as a "Heads-Up Display" (HUD) for the modern developer. It is not merely a monitor; it is a **Sovereign Substrate** where every functional domain is physically decoupled and architecturally hardened.

### 1. Aesthetic Vision (The Matrix)
- **Realism-Scale Rain**: Katakana glyphs falling with proportional speeds based on apparent Z-depth (size/brightness/speed correlation).
- **Occlusion**: Semi-transparent black borders ("Black Boxes") that allow rain to "fall behind" metrics, prioritizing data legibility.
- **Pulse Mode**: A battery-efficient alternative using static, pulsing glyphs for <0.5% CPU impact.
- **Dynamic Themes**: Instant activation of "Classic Green," "Calm Blue," and "Alert Red" based on system state or productivity triggers.

### 2. Forensic Productivity
- **Git Delta Tracking**: Real-time display of code changes (+/- lines) over a rolling 24-hour window using the `git2` substrate.
- **Automated Backups**: Hourly auto-commits with configurable thresholds (e.g., >1000 lines).
- **AI Insights**: Contextual commit messages and code tips generated via local **LiteRT + Gemma 2 (4b)** integration.
- **Dynamic Resource Protection**:
    - **Load Throttling**: AI inference and Git delta calculations are skipped if the system load exceeds **80%** to prioritize user workflow stability.
    - **Nice Threading**: Background tasks (LiteRT inference, repository audits) are executed on **low-priority (nice) threads** to prevent GUI jitter or CPU contention on the Dell G15 Ryzen platform.
    - **Fallback Logic**: If AI inference fails or is skipped, the system defaults to a structured generic commit message to maintain history integrity.

---

## III. Architectural Governance
The project adheres to the **Sovereign AI Engineering** standards:
- **Decoupled Domains**: Metrics, Rendering, GUI, and Logic are isolated into independent modules.
- **Sovereign Line Limits**:
    - **125 Lines**: Target for all "Skeleton" and new functional modules to ensure decomposition.
    - **175 Lines**: Maximum limit for "Hardened" modules to accommodate copious documentation and validation logic.
- **Zero-Trust Input**: Every GUI parameter and configuration value is strictly bounded and sanitized before entering the rendering pipeline.
- **Performance First**: Total system impact target is **<1-3% CPU usage**, with deep optimizations for hybrid graphics (AMD iGPU + NVIDIA dGPU).

---

## IV. Inclusive Design (ASD-Friendly)
- **Readability**: High-contrast ratios (WCAG AAA / 7:1) and large monospace typography (14pt+).
- **Stability**: Minimum 500ms update intervals to prevent strobing or flicker-induced distraction.
- **Predictability**: Strict layout alignment (Left/Right) without "floating" or "scrolling" distractions.

---

## V. Security & Sovereignty
- **Local-Only**: All processing (including AI) is performed on the local machine. No external telemetry or cloud dependencies.
- **Safe Operations**: Git operations are non-destructive and failure-isolated.
- **Forensic Transparency**: All system actions are logged and auditable via the `DevJournal.md` and automated build receipts.

---

## VI. Project Soul
> *"A tool that empowers the developer by making the invisible visible—progress, performance, and presence—without ever becoming a distraction."*

---

## VII. Distribution, Maintenance & Governance

### 1. Automated Release Pipeline (CI/CD)
The project utilizes **GitHub Actions** for high-fidelity automation of the build-test-release cycle.
- **Continuous Integration**: Every push triggers an automated cargo-test suite and structural audit.
- **Production Packaging**: Releases are automatically bundled into `.deb` packages optimized for Pop!_OS, including minified binaries and stripped symbols.
- **Semantic Versioning**: Employs `major.minor.patch` versioning anchored by **Conventional Commits** (feat/fix/chore) to drive automated changelog generation and release notes.
- **Open Source Licensing**: The project is published under the **MIT License**, ensuring maximum freedom for the community while preserving developer attribution.

### 2. Client-Side Integrity & Updates
To ensure seamless sovereignty for end-users:
- **Auto-Updater**: The application performs an hourly background check against the GitHub Releases API (via `reqwest`).
- **Secure Verification**: Update payloads are verified using cryptographic hashes/signatures before execution.
- **Graceful Failure**: Updates skip gracefully in offline or high-load states to prevent system jitter.

### 3. Scalability & Forensic Batching
Designed to handle massive workspaces (100k+ files) without performance degradation:
- **Repo Throttling**: Productivity tracking batch-processes repositories at a rate of **max 5 per hour**.
- **Queue Management**: Overflows are rotated and prioritized for the subsequent hour's slot to maintain the <1% CPU target.
- **Transparent Logging**: All queued operations and Git errors are logged with user-friendly remediation steps (e.g., tray alerts for "git pull" requirements).

### 4. Attribution & Identity
The "About Us" substrate reflects the collaborative soul of the project, crediting the primary developers:
- **Lead Developer**: jwils (John Wilson)
- **AI Architect**: Grok (xAI)
