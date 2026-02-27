# Implementation Checklist

## Core Infrastructure
- [ ] **Crate Integration**: `x11rb` (X11), `cairo-rs`/`pangocairo` (Graphics), `sysinfo` (Metrics), `nvml-wrapper` (NVIDIA), `tray-icon` (UI).
- [ ] **Threading Model**: Split-thread architecture (Main Event Loop vs. Metrics Collection Thread) using `crossbeam-channel`.

## Window Management (X11)
- [ ] **Multi-Monitor Support**: Create one unmanaged window per active monitor via RandR.
- [ ] **Visual Setup**: 32-bit ARGB Visual for transparency.
- [ ] **EWMH Atoms**:
    - [ ] `_NET_WM_WINDOW_TYPE_DESKTOP`
    - [ ] `_NET_WM_STATE_BELOW`
    - [ ] `_NET_WM_STATE_SKIP_TASKBAR`
    - [ ] `_NET_WM_STATE_SKIP_PAGER`
    - [ ] `_NET_WM_STATE_STICKY`
- [ ] **Input Passthrough**: XShape extension to set empty input region.
- [ ] **Layering**: Ensure correct stacking order (above wallpaper, below icons).

## Rendering Pipeline
- [ ] **Double Buffering**: Offscreen Cairo `ImageSurface` -> X11 Window.
- [ ] **Visual Style**: "Matrix-style" aesthetic (Monospace font, Green text).
- [ ] **Glow Effect**: Multi-pass rendering (offsets with low alpha).
- [ ] **Optimization**: Render diffing (skip drawing if metrics haven't changed).

## Metrics Collection
- [ ] **System Stats**: CPU Usage, RAM Usage (via `sysinfo`).
- [ ] **GPU Stats**: NVIDIA GPU Temp/Util (via `nvml-wrapper`).
- [ ] **Sensors**: Hardware monitor parsing (hwmon) for temps/fans.
- [ ] **Weather**: Open-Meteo API integration.

## Configuration & UX
- [ ] **Config File**: Configuration for colors, fonts, and layout.
- [ ] **System Tray**: Status icon with "Quit" option.
- [ ] **Global Hotkey**: Toggle visibility (e.g., Ctrl+Alt+W).

# Architecture Document: X11 System Monitoring Overlay (Rust)

**Target System:** Pop!_OS 22.04 LTS (GNOME 42.9 / Mutter / X11)
**Language:** Rust (2021 edition or later)
**Date:** 2026 (Projected)

## 1. Overview

This document outlines the architecture for a lightweight, native X11 system monitoring overlay. The application draws system metrics (CPU, Memory, GPU, Network) directly onto the desktop background, sitting visually between the wallpaper and desktop icons. It employs a "Matrix-style" aesthetic with glowing text and avoids heavy GUI frameworks (GTK/Qt) in favor of low-level X11 protocols and Cairo rendering for minimal resource usage.

## 2. Tech Stack & Crate Selection

To ensure low overhead and stability on the X11 protocol, we utilize the `x11rb` crate for direct X server communication. Rendering is handled by `cairo-rs` and `pangocairo` for high-quality text rasterization.

### Core Crates (Approx. Stable Versions 2026)

| Crate | Version | Purpose |
| :--- | :--- | :--- |
| **`x11rb`** | `~0.13` | X11 protocol bindings (Rust-native). Handles window creation, atoms, and RandR. |
| **`cairo-rs`** | `~0.21` | 2D graphics library for drawing text and effects. |
| **`pangocairo`** | `~0.21` | Text layout and rendering engine (essential for font handling). |
| **`sysinfo`** | `~0.38` | Cross-platform system information gathering (CPU, RAM, Swap). |
| **`nvml-wrapper`** | `~0.11` | NVIDIA Management Library wrapper for GPU stats (temp, util, VRAM). |
| **`serde`** | `~1.0` | Serialization framework for config parsing. |
| **`toml`** | `~0.8` | Configuration file format. |
| **`tray-icon`** | `~0.14` | System tray integration (status icon, quit menu). |
| **`global-hotkey`** | `~0.5` | Global keyboard shortcuts (e.g., Ctrl+Alt+W). |
| **`anyhow`** | `~1.0` | Error handling. |
| **`crossbeam-channel`** | `~0.5` | High-performance MPC channels for thread communication. |

## 3. Window Management Strategy (X11)

The application creates **one unmanaged window per monitor**. This avoids the complexity of a single spanning window and handles dynamic monitor plugging/unplugging via the RandR extension.

### 3.1. Window Attributes & Atoms
To achieve the "desktop overlay" effect on GNOME/Mutter:

1.  **Visual:** 32-bit ARGB Visual is required for transparency.
2.  **Window Type:** `_NET_WM_WINDOW_TYPE_DESKTOP`. This tells the Window Manager (Mutter) to treat the window as part of the desktop.
3.  **State:**
    *   `_NET_WM_STATE_BELOW`: Keeps the window at the bottom of the stack.
    *   `_NET_WM_STATE_SKIP_TASKBAR`: Hides from the taskbar/dock.
    *   `_NET_WM_STATE_SKIP_PAGER`: Hides from the Alt+Tab switcher.
    *   `_NET_WM_STATE_STICKY`: Ensures visibility on all workspaces.
4.  **Input Passthrough:** We use the **XShape** extension to set the window's input region to empty (`Rect(0, 0, 0, 0)`). This ensures all mouse clicks pass through to the desktop icons or wallpaper beneath.

### 3.2. Layering & Mutter Specifics
On GNOME 42 (Mutter), desktop icons are typically handled by an extension (e.g., Desktop Icons NG - DING). DING usually creates a window that sits just above the root window but below normal apps.

**Stacking Order:**
1.  Root Window (Wallpaper)
2.  **Overlay Window (This App)** (`_NET_WM_WINDOW_TYPE_DESKTOP`)
3.  Desktop Icons (DING) / Normal Windows

*Note: We must ensure our window does not obscure DING. By using `_NET_WM_WINDOW_TYPE_DESKTOP` and `_NET_WM_STATE_BELOW`, Mutter should stack us appropriately. If DING draws an opaque background, it may hide this overlay. DING must be configured for transparency or this overlay must be positioned to avoid icon areas.*

## 4. Rendering Pipeline

Rendering is event-driven but capped at a low framerate (≤ 1 FPS) to save power.

1.  **Double Buffering:** Use a Cairo `ImageSurface` (offscreen) for drawing, then paint the result to the X11 window surface (`XCB` surface or similar via cairo-xlib). This eliminates flickering.
2.  **Clear:** Paint the surface with `(0, 0, 0, 0)` (fully transparent) or a semi-transparent black fill depending on config.
3.  **Text Rendering (The "Glow"):**
    *   **Font:** Pango monospace (DejaVu Sans Mono or Courier), size 14pt+.
    *   **Glow Effect:**
        1.  Set source color to `#00FF41` (Matrix Green) with low alpha (e.g., 0.3).
        2.  Draw text at `(x-1, y)`, `(x+1, y)`, `(x, y-1)`, `(x, y+1)` to simulate bloom.
        3.  Set source color to `#00FF41` (Full Alpha).
        4.  Draw text at `(x, y)`.
4.  **Layout:** Text is positioned absolutely based on config, supporting alignment (Left/Right) relative to screen edges.

## 5. Data Flow & Concurrency

The architecture uses a split-thread model to decouple data collection from rendering.

### 5.1. Pseudocode

```rust
// Main Thread
fn main() {
    let config = load_config("config.toml");
    let (tx, rx) = crossbeam_channel::bounded(1);
    
    // Spawn Metrics Thread
    std::thread::spawn(move || metrics_loop(tx, config.polling_rate));
    
    // Connect to X11
    let (conn, screen_num) = x11rb::connect(None)?;
    let monitors = detect_monitors(&conn);
    let windows = create_windows_for_monitors(&conn, monitors);
    
    setup_tray_icon();
    setup_global_hotkey();

    // Event Loop
    loop {
        select! {
            recv(rx) -> metrics => {
                if metrics != last_metrics {
                    for win in windows {
                        draw_overlay(win, &metrics);
                        conn.flush();
                    }
                    last_metrics = metrics;
                }
            },
            default => {
                handle_x11_events(&conn); // Handle RandR changes, Expose events
            }
        }
    }
}

fn metrics_loop(tx, rate) {
    let mut sys = System::new_all();
    let mut nvml = Nvml::init()?;
    
    loop {
        sys.refresh_all();
        let gpu_data = nvml.device_by_index(0).map(|d| d.utilization_rates());
        
        let data = Metrics {
            cpu: sys.global_cpu_info().cpu_usage(),
            ram: sys.total_memory() - sys.used_memory(),
            gpu: gpu_data,
            // ... hwmon parsing ...
        };
        
        tx.send(data);
        sleep(rate); // e.g., 1000ms
    }
}
```

## 6. Configuration & Extensibility

Configuration is loaded from `~/.config/desktop-overlay/config.toml`.

*   **Structure:**
    *   `[general]`: Polling rate, global font settings.
    *   `[style]`: Colors (primary, glow, background), font family.
    *   `[monitors]`: List of monitors by name (e.g., "DP-1").
    *   `[[monitors.widgets]]`: List of widgets (CPU, RAM, GPU, Custom Command) with `x`, `y`, `anchor` (top-left, bottom-right).

**Future Hooks:**
*   **Open-Meteo:** A dedicated struct in the `Metrics` payload populated via `reqwest` (async runtime might be needed, or blocking call in separate thread) to fetch JSON weather data.

## 7. Performance & Privacy

*   **Privacy:** All data collection is local. No telemetry is sent. `sysinfo` reads `/proc`, `nvml` reads driver stats.
*   **Performance:**
    *   **Polling:** ≤ 1 Hz.
    *   **Diffing:** The main loop compares new metrics with the previous frame. If values (rounded to integer precision) haven't changed, rendering is skipped.
    *   **Resources:** Expected memory usage < 30MB. CPU usage < 0.5% on modern cores.

    *   **Resources:** Expected memory usage < 30MB. CPU usage < 0.5% on modern cores.