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
