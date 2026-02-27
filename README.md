# matrix-overlay

A lightweight, native X11 system monitoring overlay written in Rust. Designed for minimal resource usage and "Matrix-style" aesthetics.

## Compilation Instructions

Ensure you have Rust installed (stable).

1.  **Automated Setup**:
    Run the provided setup script to install dependencies and the binary:
    ```bash
    chmod +x install_prereqs.sh
    ./install_prereqs.sh
    ```

2.  **Manual Build**:
    If you prefer to build manually after installing dependencies (`libxcb1-dev`, `libcairo2-dev`, `libpango1.0-dev`, `libayatana-appindicator3-dev`, `lm-sensors`, `fonts-dejavu-core`, `x11-xserver-utils`, `libssl-dev`, `pkg-config`, `libxdo-dev`):
    ```bash
    cargo check
    cargo build --release
    ```

## Configuration

The application loads configuration from `~/.config/matrix-overlay/config.json`.

### Example `config.json`

```json
{
  "general": {
    "font_size": 14,
    "color": "#00FF41",
    "update_ms": 1000
  },
  "screens": [
    {
      "metrics": ["cpu_usage", "ram_usage"],
      "x_offset": 20,
      "y_offset": 120
    }
  ]
}
```

## Run Instructions

Run the binary directly. Set `RUST_LOG` to see debug output.

```bash
RUST_LOG=info ./target/release/matrix-overlay
```

To enable verbose debug logging:
```bash
RUST_LOG=debug ./target/release/x11-monitor-overlay
```

## Hardware Notes: Dell G15 5515

This tool is optimized for the Dell G15 5515 (Ryzen 7 5800H + RTX 3050 Ti) running Pop!_OS 22.04 (X11).

### Sensor Paths
- **CPU**: Uses `k10temp` driver. Path: `/sys/class/hwmon/hwmon*/name` (content: `k10temp`).
- **iGPU**: Uses `amdgpu` driver. Path: `/sys/class/hwmon/hwmon*/name` (content: `amdgpu`).
- **dGPU**: Uses proprietary NVIDIA driver via `nvidia-smi` or NVML.

### Hybrid Graphics (Prime)
On hybrid setups, the iGPU (AMD) usually handles the X11 desktop composition (Mutter), while the dGPU (NVIDIA) may be sleeping.
- **Quirk**: Querying `nvidia-smi` when the dGPU is in D3 sleep can cause system stutters.
- **Fix**: The metrics thread handles this asynchronously, but be aware of potential wake-up latency.

## Known Issues & Fixes

### 1. Layering & Click-Through
**Issue**: Window covers icons or intercepts clicks.
**Fix**: The app sets `_NET_WM_WINDOW_TYPE_DESKTOP` and clears the XShape input region.
**Verification**:
Run `xprop` and click the overlay. It should select the desktop/wallpaper behind it. If it selects the overlay window, check:
```bash
xprop -id <WINDOW_ID> | grep _NET_WM_WINDOW_TYPE
# Should output: _NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_DESKTOP
```

### 2. Flicker on Updates
**Issue**: Text flickers or tears during updates.
**Fix**: We use double-buffered rendering (Cairo ImageSurface -> X11 Pixmap).
**Verification**: Watch the "uptime" seconds counter. It should update smoothly without the background flashing.

### 3. Config Reload & Uniqueness
**Test**:
1. Run the app.
2. Edit `config.toml` (e.g., change `primary_color` to `#FF0000`).
3. Click the tray icon -> "Reload Config" or send `SIGUSR1`.
4. **Verify**: Text turns red immediately.
5. **Verify**: Check logs for "Monitors X and Y have low content uniqueness" if you configure identical metrics on both screens.

## Testing

### Layering Verification
To verify that the overlay windows are created correctly and sit below other windows:

```bash
cargo run --release -- --test-layering
```

This command will create the overlay windows and sleep for 10 seconds, allowing you to use `xprop` or `xwininfo` to inspect the window properties (e.g., `_NET_WM_WINDOW_TYPE`).

## Troubleshooting & Early Issues

If the application fails to start or compile:

1.  **Missing Dependencies**: Ensure `libxcb-shape0-dev`, `libxcb-xfixes0-dev`, and `libxcb-render0-dev` are installed.
2.  **X11 Connection**: If running in a container or headless environment, ensure `DISPLAY` is set.
3.  **Permissions**: `xsetroot` requires access to the X server.
4.  **Weather Privacy**: If weather metrics are missing, check `config.json` (or `config.toml`) and ensure `weather.enabled` is true. It defaults to `false` for privacy.

## Next Steps (Stage 2 Hooks)

The current version is a Stage 1 skeleton. Stage 2 will integrate:
1.  **Window Management**: Replacing placeholders with `src/window.rs` logic for transparent, click-through windows.
2.  **Rendering**: Connecting `src/render.rs` to draw the Matrix-style text using Cairo/Pango.
3.  **Metrics**: Connecting `src/metrics.rs` to replace the stub thread with real system data.

## Extensibility: Adding a New Metric

To add a custom metric (e.g., "Battery Level"):

1.  **Define ID**: Add `BatteryLevel` to `enum MetricId` in `src/metrics.rs`.
2.  **Implement Collector**:
    ```rust
    struct BatteryCollector { id: MetricId }
    impl MetricCollector for BatteryCollector {
        fn id(&self) -> &'static str { "battery_level" }
        fn label(&self) -> &'static str { "BAT" }
        fn collect(&mut self) -> MetricValue {
            // Read /sys/class/power_supply/BAT0/capacity
            if let Ok(cap) = fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
                if let Ok(val) = cap.trim().parse::<f64>() {
                    return MetricValue::Float(val);
                }
            }
            MetricValue::None
        }
    }
    ```
3.  **Register**: Add to `MetricsManager::new` in `src/metrics.rs`:
    ```rust
    MetricId::BatteryLevel => collectors.push(Box::new(BatteryCollector { id })),
    ```
4.  **Configure**: Add `{ id = "battery_level", ... }` to your `config.toml`.

## Performance Benchmarks

**Target Metrics** (Ryzen 5800H + RTX 3050 Ti):

1.  **CPU Usage**: **< 0.5%** idle/background usage.
    *   *Verification*: `htop` or `pidstat -p $(pgrep matrix-overlay) 5 3`.
2.  **Update Latency**: **< 1000ms** (Total time from metric collection to pixel draw).
    *   *Verification*: Check debug logs for "Metrics loop took X ms".
3.  **Memory**: < 50MB Resident Set Size (RSS).

## Verification Checklist & Testing Plan

Use this checklist to verify fixes for alignment, rendering, and layering.

### 1. Restart & Execution
Ensure the application starts cleanly.

```bash
# Kill existing instance
pkill -f matrix-overlay

 # Start with debug logging to verify monitor detection
RUST_LOG=debug ./target/release/matrix-overlay
```

### 2. Visual Verification
*   [ ] **Dual-Monitor Alignment**: Check both screens. The overlay should be positioned at the configured offsets (default `20,120`) relative to the *monitor's* top-left corner.
    *   *Check*: Ensure no "double shifting" (e.g., Monitor 2 overlay shifted right by Monitor 1's width + offset).
*   [ ] **Day of Week**: Should be **Large, Bold, and Centered** at the top of *each* monitor's overlay window.
*   [ ] **Metric Pairs**: Verify full metric pairs (CPU%, RAM%, Temps, Network) are displayed below the header.
*   [ ] **Glow Effect**: Text should have a visible green halo (Matrix style).
*   [ ] **Icon Covering**: Ensure desktop icons are **clickable** and not obscured by the overlay window (requires `_NET_WM_STATE_BELOW`).

### 3. Geometry Check
Use `xwininfo` to verify exact placement if alignment looks off.

```bash
# 1. Find Window IDs
xdotool search --name "matrix-overlay"

 # 2. Check Geometry (Repeat for each ID)
xwininfo -id <WINDOW_ID>
```
*   **Expectation**: `Absolute upper-left X` should match `Monitor X + x_offset`.

### 4. Performance & Hardware
*   [ ] **CPU Usage**: Should be **< 0.5%**. Check with `top` or `htop`.
*   [ ] **Hybrid AMD/NVIDIA**:
    *   **iGPU**: Temps read from `/sys/class/hwmon` (amdgpu).
    *   **dGPU**: Temps read via `nvidia-smi`.
    *   *Note*: If `nvidia-smi` causes stutter, disable it in config or check power management settings.

### 5. Configuration Tweaks
If text overlaps icons or alignment is incorrect:
1.  Edit `~/.config/matrix-overlay/config.json`.
2.  Safely increase `x_offset` or `y_offset` (e.g., `120` -> `150`).
3.  Restart the binary to apply.

## ASD-Friendly Design

*   **No Blinking**: Updates are smooth replacements.
*   **High Contrast**: >7:1 contrast ratio (Green on Black).
*   **Predictable**: Static layout; values update in place.
*   **Silence**: No audio cues.