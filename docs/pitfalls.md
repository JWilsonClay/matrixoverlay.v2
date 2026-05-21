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

### OverrideRedirect Pitfall (CONFIRMED BUG — 2026-05-21)
**Never set `OverrideRedirect(true)` on the overlay window.** This bypasses the window manager entirely. All EWMH atoms (`_NET_WM_WINDOW_TYPE_DESKTOP`, `_NET_WM_STATE_BELOW`) are silently ignored, and the window stacks above everything — including application windows. The overlay appears "in front of everything" instead of on the desktop. The correct value is `OverrideRedirect(false)`.

### Operator::Clear Pitfall (CONFIRMED BUG — 2026-05-21)
**Do not use `Operator::Clear` in the `clear()` rendering step.** It sets the background to fully transparent (ARGB 0,0,0,0), removing the black backing. The correct operator is `Operator::Source` with `set_source_rgba(0.0, 0.0, 0.0, 1.0)`. These two regressions were introduced by a prior AI session and are subtle — the overlay can still render metrics correctly while appearing broken in layering and background.

## MIT-SHM Pitfalls

### SHM Race Condition: Cairo Writes Directly Into SHM (CONFIRMED BUG — 2026-05-21)
When using `ImageSurface::create_for_data(ShmBuffer, ...)`, Cairo paints **directly into the SHM region** — there is no copy. The X server reads from the same region asynchronously after `ShmPutImage`. If the next frame's `clear()` fires before the X server has finished reading, it overwrites the buffer with black mid-read. The X server displays a corrupted frame (flash to black, partial content, or rain stutter).

**Fix**: Before Cairo touches the buffer (before `clear()`), issue a synchronous round-trip to confirm the X server has finished:
```rust
// In Presenter::pre_draw() for ShmPresenter:
let cookie = conn.send_request(&x::GetInputFocus {});
conn.wait_for_reply(cookie)?;
```
By the time the reply arrives, the X server has sequentially processed all prior requests including the `ShmPutImage`. The SHM region is safe to overwrite. Round-trip overhead on a local X socket is <1ms.

### SHM Drop Ordering (CRITICAL)
The Cairo `ImageSurface` holds a live pointer into the SHM region. `shmdt()` must never be called while the surface is alive. In `Drop`:
1. `self.surface.take()` — drop Cairo surface first
2. `conn.send_request(&xcb::shm::Detach { ... })` — unregister from X server
3. `libc::shmdt(self.shmaddr)` — unmap local address range

Store the surface as `Option<ImageSurface>` to enable `take()` in `Drop`.

### SHM PutImage: format is u8, not ImageFormat
`xcb::shm::PutImage::format` is typed as `u8`. Use `2u8` for ZPixmap. The `x::ImageFormat::ZPixmap` enum variant does **not** coerce here — it will not compile.

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
