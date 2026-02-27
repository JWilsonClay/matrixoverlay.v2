# Hardware Notes & Pitfalls: Dell G15 5515

## 1. Hardware Specifics
**Target System**: Dell G15 5515 (Ryzen 7 5800H + RTX 3050 Ti Mobile + AMD iGPU)

### CPU: AMD Ryzen 7 5800H
- **Driver**: `k10temp` kernel module.
  - `temp1_input` (Tctl): Instantaneous temperature, erratic.
  - `temp2_input` (Tdie): Die temperature, generally more stable.
- **Fans**: Dell laptops often hide fan control/sensing from standard `hwmon`. Requires `dell-smm-hwmon` module, but often conflicts with BIOS fan control. If `/sys/class/hwmon/hwmon*/fan1_input` is missing, fan stats may be unavailable without `i8kutils`.

### GPU 1: AMD Radeon Graphics (Integrated)
- **Driver**: `amdgpu`
- **Role**: Handles X11 desktop composition (Mutter) in hybrid mode.
- **Hwmon**: Search for `amdgpu` in `name`.

### GPU 2: NVIDIA GeForce RTX 3050 Ti Mobile
- **Driver**: Proprietary NVIDIA (e.g., 535.x, 550.x).
- **Querying**:
  - **NVML**: Preferred via `nvml-wrapper` crate.
  - **CLI**: `nvidia-smi --query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits`
- **Hybrid Mode Pitfall**: In "On-Demand" mode, the dGPU sleeps (D3 state). Querying it via `nvidia-smi` can wake it up, causing system-wide micro-stutters. `nvml` is generally lighter but still may incur wake-up latency.

## X11 & Mutter Pitfalls

### Layering & Input
- **Layering**: To sit *below* desktop icons (handled by DING/Nautilus) but *above* wallpaper:
  - Atom: `_NET_WM_WINDOW_TYPE_DESKTOP`
  - State: `_NET_WM_STATE_BELOW`
- **Click-Through**: Essential for a desktop overlay.
  - **Mechanism**: XShape extension (`xcb_shape_rectangles` or `x11rb::protocol::shape`). Set the Input region to an empty list of rectangles.
  - **Verification**: `xprop -id <window_id>` should show `_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_DESKTOP`.

### Hybrid Graphics (Prime)
- **Flicker**: Rendering to an X11 window on the dGPU while the iGPU handles composition can cause tearing.
- **Mitigation**: Ensure the overlay window is created on the screen/CRTC driven by the compositor. Use software double-buffering (Cairo ImageSurface -> X11 Pixmap -> Window) to decouple rendering from display scanout.

### Animations & ASD Considerations
- **ASD-Friendly**:
  - **High Contrast**: Use bright green/white on semi-transparent black.
  - **Stability**: No flashing or blinking elements.
  - **Refresh Rate**: 1Hz or 0.5Hz is sufficient. Avoid 60fps animations to prevent distraction and battery drain.
  - **Scroll**: If text overflows, slow scrolling is acceptable, but static text is preferred.

## Implementation Plan

### Starter `main.rs` Skeleton
The application structure ties together configuration, metrics collection, and the X11 event loop.

```rust
fn main() -> Result<()> {
    // 1. Load Config
    let config = Config::load()?;

    // 2. Setup Channels
    let (tx, rx) = crossbeam_channel::unbounded();

    // 3. Spawn Metrics Thread
    // Runs in background, sleeps for config.refresh_rate_ms
    metrics::spawn_metrics_thread(config.metrics, tx);

    // 4. Initialize X11 Window Manager
    // Connects to X server, detects monitors, creates transparent windows
    let mut wm = WindowManager::new()?;

    // 5. Initialize Renderer
    // Creates Cairo surfaces
    let mut renderer = Renderer::new(wm.width, wm.height)?;

    // 6. Event Loop
    loop {
        // Non-blocking check for X11 events (resize, expose)
        while let Some(event) = wm.poll_event() {
            handle_x11_event(event, &mut wm, &mut renderer);
        }

        // Check for new metrics
        if let Ok(data) = rx.try_recv() {
            let text = format_metrics(&data);
            renderer.draw(&wm, &text)?;
        }

        // Sleep briefly to avoid busy loop (e.g., 100ms)
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
