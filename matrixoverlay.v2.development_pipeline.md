# Development Pipeline - Blueprints v1

## Global Project Context (copy this into every stage/role prompt)
Project Name: Matrix Overlay v2
Overall Goal: Enhance desktop overlay with productivity tracking and cosmetics while maintaining low resources.
Languages Used: Rust
Key Libraries/Frameworks: xcb, cairo, sysinfo, git2, tray_icon, reqwest/ollama-rs
Database / Storage: None (config.json, local Git)
Frontend / GUI (if any): X11 overlay with tray icon
Current Architecture Summary: Modular Rust app (metrics, render, tray); adding productivity module for Git/AI.
Last Major Change: Integrated features and pipeline enhancements.

## Refined Role Starters (see role_starters.md for the three clean role prompts)

## Stage 0: Requirements & Planning (expanded for robustness)

## Features for Matrix Overlay v2 (Itemized from Discussions)

## Deployment and Publishing Features for Matrix Overlay v2 (Greenlit Enhancements)

### High Priority (Core Publishing Automation)
- **GitHub CI/CD Deployment Pipeline**: Use GitHub Actions for automated build/test/release workflows (e.g., on push: Compile Rust via Cargo, run tests from Stage 5, package binaries like .deb for Pop!_OS, upload to GitHub Releases). Include auto-changelogs from commits (tie to Ollama messages/Conventional Commits). Rationale: Streamlines releases for free publishing; ensures quality before deploy. Tie to Stage 0 (Planning) for workflow YAML setup and Stage 5 (Testing) for CI integration.

### Medium Priority (Client-Side Updates)
- **Auto-Updater Feature**: App checks GitHub Releases hourly (via reqwest API) for new versions, notifies via tray ("v2.1 available—update?"), auto-downloads/installs (e.g., fetch .deb, run dpkg or self_update crate). Toggleable/opt-in; secure verification (hashes/signatures). Rationale: Enables seamless updates for users; matches your assumption for convenience. Load: Negligible (<0.1% CPU). Integrate in Stage 3 (Debugging) for error handling (e.g., offline skips) and Stage 4 (Security) for verification.

### Low Priority (Optional Polish)
- **Extended Publishing Tools**: Add licensing (MIT for free/open-source), high-level README from Global Context, and semantic versioning (major.minor.patch). Auto-generate release notes from git log. Rationale: Prepares for GitHub repo launch; user-friendly for contributors. Implications: All configurable (e.g., update frequency). Edge: Handle install permissions (prompt sudo if needed).

Rationale for These Features: Combines standard GitHub deployment (CI/CD for automation) with client auto-updater for end-user ease, providing practice for your free launches. Focuses on low-resource, newbie-friendly implementation (no servers needed). Prioritize CI/CD for backend efficiency. Risks: API limits—mitigate with hourly caps. Success Metrics: Successful auto-builds, seamless updates, easy community contributions.

## Usability and Integration Features for Matrix Overlay v2 (Greenlit Enhancements)

### High Priority (Core Usability Upgrades)
- **Metric Repositioning and Per-Monitor Dynamics**: Make all metric positions customizable (e.g., x/y coords, order via drag/drop or config arrays). Defaults match v1 screenshot (e.g., left-aligned metrics on monitor 1). Expose v1's per-monitor config (from config.rs/screens: metrics lists, offsets) in settings with logical paths (e.g., "screens[0].metrics_order", "positions.metric_id.x/y"). Instant reload on changes. Rationale: Enhances flexibility for multi-monitor setups; preserves your preferred layout on upgrade. Tie to Stage 1 (Structure) for layout.rs updates and Stage 8 (Integration) for monitor detection.

### Medium Priority (Interactive and Feedback Elements)
- **Dynamic Themes**: Configurable color schemes (e.g., green classic, blue calm, red alert) for overlay elements (metrics, rain, borders). Toggleable; tie to time/weather/productivity (e.g., dim at night via metrics). Instant activation. Rationale: Personalizes aesthetics; low-load (variable swaps). Implications: All togglable for resource control. Integrate in Stage 2 (Functional Correctness) for theme switching tests.
- **Interactive Elements**: Add hover/click on metrics for details (e.g., click delta for repo breakdown; subtle glow on interaction). Toggleable; use XCB events for input. Rationale: Boosts engagement without CLI; optional for minimalism. Load: Low idle (+2% on use). Edge: Non-mouse fallback via tray. Prototype in Stage 3 (Debugging) for event handling.
- **Notifications Pop-ups**: Tray bubbles for events (e.g., "Auto-commit: +50 lines"). Configurable frequency/position (monitor 1 only, lower left, boxed 1/3 width, top to 40% height; "Notifications" header with list below). Toggleable; instant. Rationale: Provides feedback; groups non-essential info. Load: Negligible. Usability: Logical settings path (e.g., "notifications.enabled/position").

### Low Priority (Optional Polish)
- **AI-Generated Insights**: Periodic Ollama calls (qwen2.5) for code tips (e.g., "High delta—refactor?"). Display on monitor 1 only, right of notifications (same box proportions: 1/3 width, 40% height top; "AI Insights" header). Throttled like commits; toggleable, instant reload. Rationale: Enhances learning for new devs; fallback to off. Implications: All customizable (frequency, position). Add to Stage 5 (Testing) for AI output mocks and Stage 6 (Optimization) for load checks.
- **Extended Toggles and Realism**: Make all features togglable (e.g., "themes.enabled", "interactives.hover_glow"). Group in settings with paths (e.g., "monitors[1].notifications.box_style"). Defaults: Off for low-load; preserve v1 positions. Rationale: Maximizes efficiency/customization across machines. Edge: Validate configs to prevent invalid positions.

Rationale for These Features: Builds v2's usability (interactive, feedback-focused) while integrating v1 dynamics (per-monitor via window.rs/config.rs). All togglable for load control; instant changes aid learning. Prioritize repositioning for core flexibility. Risks: Event overhead—mitigate with toggles. Success Metrics: Smooth multi-monitor, no readability loss, user-configurable without restarts.

## Cosmetic Features for Matrix Overlay v2 (Greenlit Enhancements)

### High Priority (Core Aesthetic Upgrades)
- **Subtle Matrix Rain Effect**: Add background raining text (Katakana glyphs like in *The Matrix*, using fonts e.g., "Noto Sans CJK") with 0-5 streams entering randomly from the top, spaced apart. Randomize positions, sizes (4-12pt, smaller than metrics' 14pt), and apparent z-depth (smaller/dimmer/slower for "farther"; larger/brighter/faster for "closer"). Compute proportional falling speeds (speed = base * (size/max)^k, k=0.5-1) for realism. Reuse glow_passes (dimmed proportionally). Rationale: Enhances thematic immersion without distraction. Tie to Stage 1 (Structure) for RainManager struct and Stage 6 (Optimization) for low-load redraws.

### Medium Priority (Readability and Alternatives)
- **Black Box Borders for Metrics**: Draw semi-transparent black rectangles (configurable: square/rounded edges, with/without subtle glow) around key metric displays (from layout.rs) to occlude rain, ensuring rain "falls behind" without interfering. Render order: Rain first, then boxes/metrics. Rationale: Prioritizes functional info readability; customizable for user preference. Integrate in Stage 2 (Functional Correctness) for occlusion tests and Stage 8 (Integration) for multi-monitor.
- **Pulsate/Fade Mode Alternative**: For lower load, use static glyphs that pulse-glow (brighten/fade over 1-2s, dimmer for "far"), pause invisible for 5s, then reposition downward and repeat. Rationale: Reduces continuous animation; fallback if full rain impacts performance. Implications: Toggle in config for battery-saving; still illusions depth via glow/size.

### Low Priority (Optional Polish)
- **Extended Customizations for Rain**: Configurable options: Toggle on/off, mode ("fall" vs. "pulse"), realism scale (0-10: adjusts streams/density/glow/trails), frames per minute (60-3600 for throttle), speed multiplier (faster overall while maintaining proportional realism), letter count (more for denser rain), glow brightness (brighter for immersion), trail length (longer via alpha blending). Instant activation on toggle (reload render without exit) for real-time learning/usability. Rationale: Maximizes dev customization; lower scales reduce load across machines. Add to Stage 5 (Testing) for edge cases (e.g., high FPM lag) and Stage 7 (Documentation) for config examples.
- **Productivity-Tied Effects (Optional)**: Subtly link rain glow/speed to delta metric (e.g., brighter/faster on high productivity). Rationale: Motivates coding; low-cost tie-in. Edge: Disable if distracting.

Rationale for These Features: Builds on v1's matrix theme with fancy visuals, prioritizing low CPU/RAM (<3% added max, <0.5% in pulse mode) via Cairo optimizations (dirty rects, throttled redraws). Customizability ensures efficiency across devs' machines; instant toggles enhance usability/learning. Risks: Motion accessibility—mitigate with off-by-default. Success Metrics: Unnoticeable load (monitor via sysinfo), readable metrics, positive user feedback on immersion.

## Pipeline Enhancements and Missing Items for Matrix Overlay v2

### Enhancements to Existing Stages
- **Version Control Integration (Add to Stage 7: Documentation & Maintainability)**: Explicitly include Git/backup processes in documentation checklists, e.g., "Document commit workflows, including auto-commits and error handling." Rationale: Ensures Git features (like delta tracking) are versioned properly; helps future users/contributors. Tie to Stage 3 (Debugging) for logging Git issues (capture error messages, provide user-friendly fix steps like "Run 'git status' or check config paths"). Edge: For published tool, add troubleshooting README section.
- **AI-Specific Workflow (Add to Stage 0: Requirements & Planning and Stage 5: Testing)**: Add prompts for AI tools (e.g., Ollama for commit messages) in role starters, e.g., "Consider AI integrations: Feasibility, prompts, fallbacks." In Stage 5, include tests for AI outputs (e.g., mock Ollama responses). Rationale: Reflects your AI-driven style; ensures qwen2.5 calls are tested for accuracy/hallucinations. Nuances: If Ollama call skipped (e.g., high load), fallback to generic commit message.
- **Resource Awareness (Enhance Stage 6: Performance / Optimization)**: Add checklists for system impact monitoring (e.g., "Verify CPU throttling via sysinfo; benchmark Ollama calls"). Include notes on background processing (e.g., low-priority threads for Ollama). Rationale: Aligns with low-resource goals on Dell G15 Ryzen; prevents surprises during auto-features. Edge: During heavy computation, skip non-essential calls entirely.

### New Additions
- **Scalability for Multi-Repo Handling (Add to Stage 6: Performance and Config.json)**: For auto-commits/Ollama, batch process repos (max 5 per hour); rotate/queue overflows (e.g., 7 repos: Backup 5, prioritize remaining 2 for next hour slots 1-2). Rationale: Handles future growth (e.g., 100k+ files across repos) without overload; ensures fair backups. Implications: Configurable cap; log queues for transparency. Edge: If no changes in queued repos, deprioritize.
- **Error Handling and User-Friendliness (Add to Stage 3: Debugging and Stage 7: Documentation)**: For Git issues (e.g., conflicts, permissions), log errors, capture messages, and provide simple fix steps (e.g., tray notifications: "Git push failed—try 'git pull' or check internet"). Rationale: Since you'll publish for free, make it accessible for non-experts; reduces frustration. Nuances: Focus on recovery (e.g., retry next hour).
- **About Us Section Update (Enhance Tray Icon Pulldown in Medium Priority Features)**: In the "About Us" submenu, credit developers: "jwils (John Wilson) and Grok (xAI)". Rationale: Acknowledges collaboration; builds community trust. Edge: Include version, license (e.g., MIT for free publish), and link to repo.

Rationale for These Enhancements: Addresses gaps in the original pipeline (e.g., Git/AI/resource focus) while incorporating user notes for robustness. Ensures v2 is scalable, user-friendly, and low-impact, especially for published tools. Prioritize: Implement in early stages to avoid retrofits. Risks: Over-complication—mitigate by keeping additions optional/toggleable. Success Metrics: Pipeline covers 100% of discussed features; no unhandled edges in testing; community-ready docs.

### High Priority (Core Productivity Enhancements)
- **Delta Lines Tracking**: Compute and display lines added/removed (+/-) over a rolling 24-hour window using local Git repos (via git2 crate). Adaptive interval: Start at 1 hour if inactive (e.g., after a day off), scale to full 24 hours as activity builds. Refresh hourly. Display in overlay as a new metric (e.g., MetricId::CodeChanges). Rationale: Provides feedback on coding progress without distraction. Tie to Stage 2 (Functional Correctness) for verification and Stage 5 (Testing) for edge cases like no activity.
- **Auto-Commits with Thresholds**: Automatically commit and push changes hourly to GitHub repos, skipping if <1000 lines changed or a manual commit occurred in the past hour. Use generic messages initially (e.g., "Matrix overlay auto-commit: YYYYMMDD HH:MM"). Opt-in via config.json. Rationale: Serves as reliable backup for busy/AI-driven workflow. Integrate in Stage 3 (Debugging) for conflict handling and Stage 4 (Security) for safe pushes.

### Medium Priority (AI Integration and Usability)
- **Ollama-Generated Commit Messages**: Use local qwen2.5 model (via Ollama) to summarize diffs since last auto-commit, generating contextual messages per repo (e.g., "Added delta tracking feature to metrics.rs"). Throttle: Max 1 call/hour, run in background at low priority (e.g., nice thread), skip if system load >80% or during heavy computation. Fallback to generic if Ollama fails. Rationale: Adds value to auto-commits without manual effort; enhances learning. Discuss in Stage 0 for feasibility, test in Stage 5, optimize in Stage 6.
- **Tray Icon Pulldown Menu**: Add submenu to green sys tray icon: "Settings" (edit config.json), "Rearrange Display Items" (change order of metrics like CPU/RAM/delta), "About Us" (version, credits). Currently, clicks do nothing—enhance for interactivity. Rationale: Improves usability for non-CLI users. Prototype in Stage 1 (Structure), review in Stage 8 (Integration).

### Low Priority (Optional Polish)
- **Auto-Commit Cleanup (Conditional)**: If not using Ollama messages, delete/revert previous auto-commit after new one (skipping manuals). Use git revert for safety. Rationale: Keeps history tidy for future community. Edge: Avoid if messages add context. Add to Stage 7 (Documentation) for history notes.

Rationale for Itemization: These features build on v1's metrics/overlay, focusing on productivity (delta/auto-backup) while respecting resource constraints (throttling, thresholds). Prioritize based on impact; estimate: 1-2 weeks implementation with AI assistance. Risks: Git errors, Ollama load—mitigate with toggles. Success Metrics: Low CPU impact (<1%), accurate deltas, no lost work.

Goal: Capture complete requirements, skill gaps, tech stack decisions, feasibility, risks, milestones, and success criteria before any code.

Recommended Expert Role: Architect (primary), then PM for breakdown

Template:
### STAGE 0: Requirements & Planning

Global Project Context:
Project Name: Matrix Overlay v2
Overall Goal: Enhance desktop overlay with productivity tracking and cosmetics while maintaining low resources.
Languages Used: Rust
Key Libraries/Frameworks: xcb, cairo, sysinfo, git2, tray_icon, reqwest/ollama-rs
Database / Storage: None (config.json, local Git)
Frontend / GUI (if any): X11 overlay with tray icon
Current Architecture Summary: Modular Rust app (metrics, render, tray); adding productivity module for Git/AI.
Last Major Change: Integrated features and pipeline enhancements.

What I Want to Build: An enhanced version of the Matrix Overlay (v2) for Pop!_OS Linux. It retains the matrix-themed system metrics display but adds productivity features: tracking code changes (delta lines) over 24h via local Git, auto-committing changes hourly with thresholds, generating commit messages using local AI (Ollama/qwen2.5), and improving the system tray menu (settings, about us). It also includes subtle cosmetic effects like matrix rain.
Skills I Already Have: Basic Rust setup, using AI for coding assistance.
Skills I Need to Learn: Git integration (git2), Ollama API integration, advanced performance optimization (threading/throttling), tray icon interactivity.
Known Constraints (time, budget, platform, etc.): Time (1-2 weeks), Platform (Linux/Pop!_OS on Dell G15 Ryzen), Budget ($0/Free tools), Hardware (Low resource impact priority).
Non-functional Requirements (performance, security level, offline support, etc.): Performance (<1% CPU/RAM added), Security (Local-only processing, safe Git ops), Scalability (Batch processing for multiple repos), Offline-friendly (Fallback for AI).
Key Features (list prioritized): 
1. Delta Lines Tracking (Git-based +/- over 24h).
2. Auto-Commits with Thresholds (<1000 lines, hourly check).
3. Ollama-Generated Commit Messages (Throttled, background thread).
4. Tray Icon Enhancements (Pulldown menu: Settings, About Us).
5. Cosmetic Enhancements (Subtle matrix rain).
6. Scalability for Multi-Repo Handling (Batching).
7. Error Handling & User Notifications.
Success Metrics: <1% CPU usage added, accurate delta tracking, reliable auto-commits without conflicts, no crashes, functional tray menu.

[Optional: paste any initial research or sketches]

Architect Version → copy to Architect LLM
[PASTE ARCHITECT ROLE STARTER HERE, fill blanks]

PM Version → copy to SeniorPM LLM (after Architect responds)
[PASTE PM ROLE STARTER HERE]

Engineer Version → copy to SeniorSE LLM (after PM responds)
[PASTE ENGINEER ROLE STARTER HERE]




## Global Project Context (copy this block into every stage/role prompt)
Project Name: Matrix Overlay v2
Overall Goal: Enhance desktop overlay with productivity tracking and cosmetics while maintaining low resources.
Languages Used: Rust
Key Libraries/Frameworks: xcb, cairo, sysinfo, git2, tray_icon, reqwest/ollama-rs
Database / Storage: None (config.json, local Git)
Frontend / GUI (if any): X11 overlay with tray icon
Current Architecture Summary: Modular Rust app (metrics, render, tray); adding productivity module for Git/AI.
Last Major Change: Integrated features and pipeline enhancements.

## Quick Iteration Mode (save this separately as quick_iteration_mode.md)
When working rapidly on the same file/module in one session, use this shortened version instead of full context:

### QUICK ITERATION - Stage #[STAGE_NUMBER]
Current File/Module: 
Specific Focus: 
Code Snippet: 
[PASTE CODE]

Architect / PM / Engineer role starters still apply, but omit full Global Project Context.

## Stage 1: Code Structure & Style

Goal: Ensure code follows language conventions, is readable, modular, and maintainable. Catch style violations early.

Recommended Expert Role: Senior Systems Engineer (heavy focus on refactoring)

Template:
### STAGE 1: Code Structure & Style

Global Project Context:
[PASTE ABOVE]

Code to Review (paste full module/file here):

Specific Style/Readability Concerns (optional):

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 1 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 1 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 1 details]

Checklist:
- Updated Global Project Context?
- Saved refactored code to file?
- Updated Last Major Change?
- Noted any new directives or decisions?

## Stage 2: Functional Correctness

Goal: Verify the code matches intended requirements and behaves correctly under normal conditions.

Recommended Expert Role: Architect + Senior Systems Engineer

Template:
### STAGE 2: Functional Correctness

Global Project Context:
[PASTE ABOVE]

Intended Behavior / Requirements Summary:
Key Functions/Modules to Verify:

Code to Analyze:

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 2 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 2 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 2 details]

Checklist:
- Updated Global Project Context?
- Saved any corrected code?
- Updated Last Major Change?
- Noted any requirement gaps found?

## Stage 3: Debugging / Specific Issues

Goal: Diagnose and fix targeted bugs or unexpected behavior.

Recommended Expert Role: Senior Systems Engineer

Template:
### STAGE 3: Debugging / Specific Issues

Global Project Context:
[PASTE ABOVE]

Specific Issue:
Error Message: 
File/Line: 
Reproduction Steps: 
Expected Behavior: 
Actual Behavior: 

Code Snippet:

What I've Already Tried:

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 3 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 3 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 3 details]

Checklist:
- Updated Global Project Context?
- Saved fixed code?
- Updated Last Major Change?
- Reproduced fix locally?

## Stage 4: Security Audit

Goal: Identify vulnerabilities, data handling issues, and security best practices gaps.

Recommended Expert Role: Architect (threat modeling) + Senior Systems Engineer

Template:
### STAGE 4: Security Audit

Global Project Context:
[PASTE ABOVE]

Specific Security Focus Areas (e.g., input validation, auth, data storage):

Code / Modules Handling Sensitive Data:

Threat Model Notes:

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 4 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 4 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 4 details]

Checklist:
- Updated Global Project Context?
- Saved security fixes?
- Updated Last Major Change?
- Reviewed dependencies/vulnerabilities?

## Stage 5: Testing Strategy

Goal: Define and implement comprehensive tests (unit, integration, edge cases).

Recommended Expert Role: Senior Systems Engineer

Template:
### STAGE 5: Testing Strategy

Global Project Context:
[PASTE ABOVE]

Key Functions/Modules to Test:
Test Types Needed (unit, integration, property-based, etc.):

Existing Tests (if any):

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 5 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 5 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 5 details]

Checklist:
- Updated Global Project Context?
- Saved test files/code?
- Updated Last Major Change?
- Ran tests successfully?

## Stage 6: Performance / Optimization

Goal: Identify bottlenecks, improve efficiency, and optimize resource usage.

Recommended Expert Role: Senior Systems Engineer

Template:
### STAGE 6: Performance / Optimization

Global Project Context:
[PASTE ABOVE]

Known Performance Concerns or Hot Paths:
Benchmarks / Profiling Data (if available):

Code to Optimize:

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 6 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 6 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 6 details]

Checklist:
- Updated Global Project Context?
- Saved optimized code?
- Updated Last Major Change?
- Verified performance improvement?

## Stage 7: Documentation & Maintainability

Goal: Add comments, docstrings, README updates, type hints, and architectural notes.

Recommended Expert Role: Senior Systems Engineer + Architect

Template:
### STAGE 7: Documentation & Maintainability

Global Project Context:
[PASTE ABOVE]

Files/Modules Needing Documentation:
Specific Documentation Needs:

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 7 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 7 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 7 details]

Checklist:
- Updated Global Project Context?
- Saved documentation updates?
- Updated Last Major Change?
- README current?

## Stage 8: Final Integration Review

Goal: Ensure the module integrates well with the overall architecture, check consistency, and prepare for next iteration.

Recommended Expert Role: Architect + Senior Project Manager

Template:
### STAGE 8: Final Integration Review

Global Project Context:
[PASTE ABOVE]

Modules Affected by Recent Changes:
Integration Points / Dependencies:

Overall Concerns:

Architect Version → copy to Architect LLM:
[PASTE ARCHITECT ROLE STARTER + fill Stage 8 details]

Senior Project Manager Version → copy to SeniorPM LLM:
[PASTE PM ROLE STARTER + fill Stage 8 details]

Senior Systems Engineer Version → copy to SeniorSE LLM:
[PASTE ENGINEER ROLE STARTER + fill Stage 8 details]

Checklist:
- Updated Global Project Context?
- Merged/integrated changes?
- Updated Last Major Change?
- Ready for next stage or release?
