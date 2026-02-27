# Feature Checklist - Matrix Overlay v2

## High Priority (Core)
- [ ] **GitHub CI/CD Deployment Pipeline**: Automated build/test/release workflows.
- [ ] **Metric Repositioning and Per-Monitor Dynamics**: Customizable metric positions and per-monitor configs.
- [ ] **Subtle Matrix Rain Effect**: Background raining text (Katakana glyphs) with depth.
- [ ] **Delta Lines Tracking**: local Git repos (+/- over 24h).
- [ ] **Auto-Commits with Thresholds**: Hourly commits to GitHub (skip if <1000 lines).

## Medium Priority (Enhancements)
- [ ] **Auto-Updater Feature**: hourly check for GitHub Releases, tray notifications.
- [ ] **Dynamic Themes**: Configurable color schemes (classic, calm, alert).
- [ ] **Interactive Elements**: Hover/click on metrics for details (glow on interaction).
- [ ] **Notifications Pop-ups**: Tray bubbles for "Auto-commit" events.
- [ ] **Black Box Borders for Metrics**: Semi-transparent rectangles to occlude rain.
- [ ] **Ollama-Generated Commit Messages**: local qwen2.5 summaries for diffs.
- [ ] **Tray Icon Pulldown Menu**: Settings, Rearrange, About Us.

## Low Priority (Polish)
- [ ] **Extended Publishing Tools**: Licensing, README, semantic versioning.
- [ ] **AI-Generated Insights**: Periodic qwen2.5 tips on code (throttled).
- [ ] **Extended Toggles and Realism**: All features toggleable in config.
- [ ] **Pulsate/Fade Mode Alternative**: Static glyphs pulse-glow for lower load.
- [ ] **Extended Customizations for Rain**: Toggle, mode, density, speed, glow.
- [ ] **Productivity-Tied Effects**: Rain glow/speed tied to delta metric.
- [ ] **Auto-Commit Cleanup**: Delete/revert previous auto-commits if no Ollama.
- [ ] **Scalability for Multi-Repo Handling**: Batch process repos (max 5/hour).
- [ ] **About Us Section Update**: Credits to jwils and Grok.
