# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

`matrix-overlay` is a native X11 desktop overlay that renders system metrics (CPU, RAM, GPU, network, weather, git activity) directly onto the desktop background in a Matrix-style aesthetic. It targets Pop!_OS 22.04 / GNOME 42 / Mutter on X11.

## Build & Run

```bash
# Debug build
cargo build

# Optimized release build (LTO, size-optimized, stripped)
cargo build --release

# Run (X11 session required)
DISPLAY=:0 ./target/release/matrix-overlay

# Run without weather feature (removes tokio/reqwest deps)
cargo build --no-default-features

# Run tests
cargo test

# Run a single test module
cargo test path_utils

# Run benchmarks (Cairo/Pango rendering)
cargo bench
```

The binary requires `DISPLAY` to be set. It will bail early with a clear error if not.

## Configuration

Config file: `~/.config/matrix-overlay/config.json` (JSON, **not** TOML).  
Auto-created with defaults on first run. Saved atomically (write to `.tmp` then rename). Permissions set to `0o600`.

`Config` uses `#[serde(deny_unknown_fields)]` on all structs. **Adding a new config field requires `#[serde(default)]`** or existing configs will fail to parse.

Available metric IDs for `screens[].metrics`: `cpu_usage`, `ram_usage`, `ram_used`, `ram_total`, `load_avg`, `uptime`, `network_details`, `disk_usage`, `cpu_temp`, `fan_speed`, `gpu_temp`, `gpu_util`, `weather_temp`, `weather_condition`, `day_of_week`, `code_delta`, `overlay_cpu`, `location_data`, or any custom string (mapped to `MetricId::Custom`).

## Architecture

### Thread Model

Five threads communicate via `crossbeam-channel` and a shared `Arc<AtomicBool>` shutdown flag:

| Thread | Role |
|--------|------|
| **Main** | GTK event loop; opens `ConfigWindow` on tray request |
| **Overlay** | Core loop: receives XCB events, tick signals, menu events, GUI events, update events; drives rendering |
| **XCB** | Blocks on `wait_for_event()`, forwards X11 events to overlay thread |
| **Metrics** | Polls all collectors on `update_ms` interval; writes to `Arc<Mutex<SharedMetrics>>` |
| **Tick** | Fires ~30fps signals to overlay thread to trigger redraws |
| **Productivity** | Hourly auto-commit cycle via `git2` + optional Ollama |
| **Update Checker** | Checks GitHub releases every 24h via `self_update` |

The overlay thread reads `SharedMetrics` via the mutex on each tick rather than receiving metric values over a channel.

### Module Map

```
src/
  main.rs               Entry point → core::version::print_startup_info() → core::main::run()
  lib.rs                Crate root re-exporting all modules
  core/
    main.rs             Orchestrates full startup sequence (8 numbered steps)
    config/             Config load/save/validate; types in types.rs, defaults in defaults.rs,
                        path-hardened I/O in storage.rs
    window/             XCB window creation (one per monitor via RandR), EWMH atoms, XShape
    threads/            Thread spawn helpers + handlers.rs (handle_xcb_event, draw_frame, etc.)
    layout.rs           Computes Layout structs (positions/sizes) for each monitor
    init.rs             XCB setup, logging init, autostart setup
    productivity/       git2-based auto-commit + Ollama AI commit message generation
    update.rs           GitHub release checking via self_update
  metrics/
    mod.rs              MetricId enum, MetricValue, SharedMetrics, MetricCollector trait
    manager.rs          spawn_metrics_thread(); drives all collectors
    dispatch.rs         init_collectors() — builds collector list from config
    factory.rs          Collector construction helpers
    collectors/
      system/           sysinfo-based: cpu, memory, network, process, storage
      hwmon.rs          /sys/class/hwmon kernel sensor parsing
      nvidia.rs         nvidia-smi subprocess parsing (NVML path)
      weather.rs        Open-Meteo HTTP fetch (tokio/reqwest, behind "weather" feature)
      git.rs            git2 repo stat collection
      ai.rs             Ollama integration
      file.rs           Custom file/tail metric sources
      date.rs           Day-of-week collector
  render/
    engine/
      renderer.rs       Renderer struct (Cairo ImageSurface + Pango font + RainManager)
      pipeline.rs       Full frame draw pipeline
      presentation.rs   Copies ImageSurface to XCB window
    layout/
      components.rs     Individual metric widget rendering
      drawing.rs        Cairo primitives (glow passes, backgrounds)
      formatting.rs     MetricValue → display string
    physics/
      rain_manager.rs   Manages RainStream pool
      rain_stream.rs    Single falling column of Matrix characters
  ui/
    tray.rs             System tray icon + context menu (tray-icon + gtk)
    gui/
      mod.rs            ConfigWindow (GTK Notebook with tabbed panels)
      logic.rs          update_config_from_widgets() — reads GTK widget state → Config
      general/cosmetics/weather/advanced/metrics/productivity.rs  Tab builders
```

### Rendering Pipeline

Each tick: `draw_frame()` → per-renderer pipeline:
1. Clear Cairo `ImageSurface` to transparent
2. Draw Matrix rain physics (`RainManager`)
3. Draw metrics panel (glow passes at offsets with low alpha, then full-alpha text)
4. Copy `ImageSurface` to XCB window via `presentation.rs`

The glow effect is multi-pass: draw text at `(x±n, y±n)` with low alpha, then once at full alpha.

### X11 Window Setup

One `xcb::Window` per monitor (detected via RandR). Each window gets:
- 32-bit ARGB visual for transparency
- EWMH atoms: `_NET_WM_WINDOW_TYPE_DESKTOP`, `_NET_WM_STATE_BELOW`, `_NET_WM_STATE_SKIP_TASKBAR`, `_NET_WM_STATE_SKIP_PAGER`, `_NET_WM_STATE_STICKY`
- XShape empty input region for full click-through
- `StackMode::Below` on startup to sit above wallpaper, below icons

### Keyboard Shortcuts (in overlay thread)

- `Ctrl+Alt+W` — toggle overlay visibility
- `Ctrl+Alt+Q` — quit

## Hardware Notes (Dell G15 5515)

- **CPU temp**: `k10temp` hwmon; prefer `temp2_input` (Tdie) over `temp1_input` (Tctl)
- **NVIDIA dGPU**: In PRIME "On-Demand" mode the dGPU sleeps; polling wakes it and may cause micro-stutters. The nvidia collector uses `nvidia-smi` subprocess
- **AMD iGPU**: Handles X11 composition; look for `amdgpu` in hwmon `name` file
- **Fan sensors**: May be unavailable without `dell-smm-hwmon` or `i8kutils`
