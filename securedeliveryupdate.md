# Walkthrough — Secure Delivery & Tray Notifications

I have successfully implemented the **Secure Delivery Substrate** for Matrix Overlay v2. This includes binary hardening, GitHub-integrated update detection, and Tray Notification signaling.

## Changes Made

### 1. The Forge (Binary Hardening)
- **Cargo.toml**: Updated the release profile with `panic = "abort"`, `strip = true`, and `opt-level = "z"` for maximum opacity and minimal size.
- **scripts/harden_build.sh**: A new production script that automates the creation of a "Closed Source" binary by stripping all symbols and (optionally) applying UPX packing.

### 2. The Sentry (Update Client)
- **src/core/update.rs**: A new module that uses the `self_update` crate to check GitHub Releases for newer versions.
- **Async Background Task**: The app now checks for updates every 24 hours (configurable) without blocking the UI.
- **Security**: Implemented a SHA-256 verification shim to ensure downloaded binaries match expected signatures.

### 3. The Herald (Tray UI)
- **src/ui/tray.rs**: Added logic to handle "Update Available" events.
- **Notification Bubbles**: Integrated `notify-rust` to provide a one-time desktop bubble when an update is discovered.
- **Menu Actions**: Added a dynamic "Update Available" cue to the system tray menu.

---

## The "Live Simulation" — Your Manual Steps

To complete this simulation and see the "Secure Delivery" in action, please follow these steps:

### 1. Prepare your GitHub Repository
Ensure your project is pushed to GitHub at `JWilsonClay/matrixoverlay.v2`. 

### 2. Create the "Hardened" Release
1. Run the hardening script: `./scripts/harden_build.sh`
2. Go to your GitHub repository in your browser.
3. Click **Releases** -> **Create a new release**.
4. Set the tag version to `v0.1.1`.
5. **CRITICAL**: Upload the binary found at `dist/matrix-overlay` as an asset to this release.
6. Publish the release.

### 3. Trigger the Update (The Simulation)
1. In your local `Cargo.toml`, change the version back to `0.1.0`.
2. Start the application natively: `cargo run`
3. The app will detect that `v0.1.1` is available on GitHub (since your local is now 0.1.0).
4. **Watch for the Bubble**: You should see a system notification saying "A new version is ready for secure delivery."
5. The application will log the discovery and (once we wire the final click) will automatically swap the binary and restart.

---

## Verification Results
- **Hardening**: Binary is successfully `stripped` (verified via `file` command).
- **Compilation**: `cargo check` passed with all new dependencies (`self_update`, `sha2`, `notify-rust`).
- **Logic**: The update thread is correctly spawned and initialized in `src/core/main.rs`.
