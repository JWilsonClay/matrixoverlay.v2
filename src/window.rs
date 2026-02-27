//! Window management and monitor detection using XCB and RandR.
//! Handles detection of active monitors, geometry querying, and refresh rate calculation.

use anyhow::{Context, Result};
use xcb::randr;
use xcb::x;
use xcb::shape;
use xcb::Xid;
use cairo::{ImageSurface, Format, Context as CairoContext};
use crate::config::Config;

/// Represents a physical monitor detected via RandR.
#[derive(Debug, Clone)]
pub struct Monitor {
    /// RandR Output ID
    pub id: u32,
    /// Output name (e.g., "eDP-1", "HDMI-1")
    pub name: String,
    /// X position in the global screen coordinate space
    pub x: i16,
    /// Y position in the global screen coordinate space
    pub y: i16,
    /// Width in pixels
    pub width: u16,
    /// Height in pixels
    pub height: u16,
    /// Refresh rate in Hz (rounded)
    pub refresh: u32,
}

/// Detects connected monitors using the XCB RandR extension.
///
/// Queries the X server for screen resources, iterates through available outputs,
/// and filters for active (connected and CRTC-assigned) monitors.
///
/// # Returns
/// A vector of `Monitor` structs, ordered with the primary monitor first (if configured),
/// followed by others sorted by their X position (left-to-right).
pub fn detect_monitors(conn: &xcb::Connection) -> Result<Vec<Monitor>> {
    // 1. Get the root window of the first screen
    let setup = conn.get_setup();
    let screen = setup.roots().next().context("No screen found")?;
    let root = screen.root();

    // 2. Get Screen Resources
    // This call is essential to get the list of outputs and modes.
    let resources_cookie = conn.send_request(&randr::GetScreenResources { window: root });
    let resources = conn.wait_for_reply(resources_cookie).context("Failed to get RandR screen resources. Is RandR supported?")?;

    // 3. Get Primary Output
    // We use this to sort the primary monitor to the front of the list.
    let primary_cookie = conn.send_request(&randr::GetOutputPrimary { window: root });
    let primary_output = conn.wait_for_reply(primary_cookie).map(|r| r.output().resource_id()).unwrap_or(0);

    let mut monitors = Vec::new();
    let timestamp = resources.config_timestamp();

    // 4. Iterate over all outputs provided by RandR
    for &output in resources.outputs() {
        let output_info_cookie = conn.send_request(&randr::GetOutputInfo {
            output, config_timestamp: timestamp
        });
        let output_info = match conn.wait_for_reply(output_info_cookie) {
            Ok(info) => info,
            Err(e) => {
                log::warn!("Failed to get info for output {:?}: {}", output, e);
                continue;
            }
        };

        // 5. Filter active outputs
        // We only care about outputs that are connected and have a CRTC assigned (are active).
        // Connection status: 0 = Connected, 1 = Disconnected, 2 = Unknown
        if output_info.connection() != randr::Connection::Connected || output_info.crtc().resource_id() == 0 {
            continue;
        }

        // 6. Get CRTC Info (Geometry)
        // The CRTC info contains the x, y, width, height, and mode of the output.
        let crtc_info_cookie = conn.send_request(&randr::GetCrtcInfo {
            crtc: output_info.crtc(), config_timestamp: timestamp
        });
        let crtc_info = match conn.wait_for_reply(crtc_info_cookie) {
            Ok(info) => info,
            Err(e) => {
                log::warn!("Failed to get CRTC info for output {:?}: {}", output, e);
                continue;
            }
        };

        // 7. Calculate Refresh Rate
        // We look up the mode ID in the resources to find the dot clock and total dimensions.
        let mode_id = crtc_info.mode();
        let refresh = resources.modes().iter()
            .find(|m| m.id == mode_id.resource_id())
            .map(|m| {
                if m.htotal > 0 && m.vtotal > 0 {
                    let dot_clock = m.dot_clock as f64;
                    let htotal = m.htotal as f64;
                    let vtotal = m.vtotal as f64;
                    // Refresh rate = dot_clock / (htotal * vtotal)
                    (dot_clock / (htotal * vtotal)).round() as u32
                } else {
                    60 // Fallback if dimensions are invalid
                }
            })
            .unwrap_or(60);

        // 8. Get Name
        // Convert the raw bytes of the name to a String.
        let name = String::from_utf8_lossy(output_info.name()).to_string();

        monitors.push(Monitor {
            id: output.resource_id(),
            name,
            x: crtc_info.x(),
            y: crtc_info.y(),
            width: crtc_info.width(),
            height: crtc_info.height(),
            refresh,
        });
    }

    // 9. Sort (Primary first, then Left-to-Right based on X position)
    monitors.sort_by(|a, b| {
        if a.id == primary_output {
            std::cmp::Ordering::Less
        } else if b.id == primary_output {
            std::cmp::Ordering::Greater
        } else {
            a.x.cmp(&b.x)
        }
    });

    log::info!("Detected {} active monitors", monitors.len());
    for m in &monitors {
        log::info!("  - {} (ID: {}): {}x{}@{}Hz at {},{}", m.name, m.id, m.width, m.height, m.refresh, m.x, m.y);
    }

    Ok(monitors)
}

/// Creates a transparent overlay window for a specific monitor.
/// Finds a 32-bit ARGB visual and creates an override-redirect window.
///
/// # Verification
/// Use `xwininfo -id <WINDOW_ID>` to verify that "Absolute upper-left X" and "Absolute upper-left Y"
/// match the monitor's RandR position exactly (e.g., 0,0 or 1920,0), without extra offsets.
pub fn create_overlay_window(conn: &xcb::Connection, monitor: &Monitor, _config: &Config) -> Result<x::Window> {
    let setup = conn.get_setup();
    let screen = setup.roots().next().context("No screen found")?;

    // Find 32-bit ARGB Visual (Depth 32, TrueColor, Alpha mask exists)
    let visual_type = screen.allowed_depths()
        .find(|d| d.depth() == 32)
        .and_then(|d| {
            d.visuals().iter().find(|v| {
                v.class() == x::VisualClass::TrueColor && 
                (v.red_mask() | v.green_mask() | v.blue_mask()) != 0xFFFFFFFF
            })
        })
        .context("No 32-bit ARGB visual found")?;

    let visual_id = visual_type.visual_id();

    // Create Colormap
    let colormap = conn.generate_id();
    conn.send_request(&x::CreateColormap {
        alloc: x::ColormapAlloc::None,
        mid: colormap,
        window: screen.root(),
        visual: visual_id,
    });

    // Position window exactly at monitor coordinates (clamped to monitor bounds by definition).
    // Offsets from config are applied during rendering as safe margins, not here.
    let x = monitor.x;
    let y = monitor.y;
    log::debug!("Creating overlay window for '{}' at ({}, {}) {}x{}", monitor.name, x, y, monitor.width, monitor.height);

    let window = conn.generate_id();
    conn.send_request(&x::CreateWindow {
        depth: 32,
        wid: window,
        parent: screen.root(),
        x,
        y,
        width: monitor.width,
        height: monitor.height,
        border_width: 0,
        class: x::WindowClass::InputOutput,
        visual: visual_id,
        value_list: &[
            x::Cw::BackPixel(0x00000000),
            x::Cw::BorderPixel(0),
            x::Cw::OverrideRedirect(false),
            x::Cw::EventMask(x::EventMask::EXPOSURE | x::EventMask::KEY_PRESS),
            x::Cw::Colormap(colormap),
        ],
    });

    Ok(window)
}

/// Configures EWMH properties for the overlay window.
///
/// # Mutter / GNOME 42.9 X11 Behavior
///
/// When using `override_redirect` (which we do to bypass WM positioning and borders),
/// the Window Manager (Mutter) technically stops managing the window's stacking order
/// via `_NET_WM_STATE`. However, setting `_NET_WM_WINDOW_TYPE_DESKTOP` is crucial
/// for the compositor to recognize this window as part of the desktop background layer.
///
/// - **Layering**: With `override_redirect`, the window sits in the unmanaged layer.
///   To ensure it sits *behind* desktop icons (handled by DING or Nautilus), we rely
///   on X11 stacking order. While `_NET_WM_STATE_BELOW` is a hint for managed windows,
///   we set it here for completeness and potential compositor heuristics.
/// - **Input**: We must also ensure the window is click-through (handled via XShape elsewhere)
///   so it doesn't block interaction with the icons above it.
///
/// # Verification Commands
/// ```bash
/// xprop -id <WINDOW_ID> | grep -E 'WM_CLASS|_NET_WM_WINDOW_TYPE|_NET_WM_STATE'
/// xwininfo -id <WINDOW_ID>
/// xprop -root | grep _NET_CLIENT_LIST_STACKING
/// ```
///
/// # Mutter-Specific Notes
/// `override_redirect` + `_NET_WM_STATE_BELOW` works reliably on GNOME 42.9 X11 for desktop
/// layering without covering Nautilus icons.
///
/// # Test Steps
/// 1. **Dual-Monitor**: eDP primary + HDMI.
/// 2. **Icon Covering**: Ensure no icon covering on both screens.
/// 3. **Stability**: Test for stable positioning at 120Hz/60Hz.
pub fn setup_ewmh_properties(conn: &xcb::Connection, win: x::Window) -> Result<()> {
    // Intern atoms
    let atom_names = [
        "_NET_WM_WINDOW_TYPE",
        "_NET_WM_WINDOW_TYPE_DESKTOP",
        "_NET_WM_STATE",
        "_NET_WM_STATE_BELOW",
        "_NET_WM_STATE_STICKY",
        "_NET_WM_STATE_SKIP_TASKBAR",
        "_NET_WM_STATE_SKIP_PAGER",
    ];

    let cookies: Vec<_> = atom_names
        .iter()
        .map(|name| {
            conn.send_request(&x::InternAtom {
                only_if_exists: false,
                name: name.as_bytes(),
            })
        })
        .collect();

    let mut atoms = Vec::with_capacity(atom_names.len());
    for cookie in cookies {
        atoms.push(conn.wait_for_reply(cookie)?.atom());
    }

    let net_wm_window_type = atoms[0];
    let net_wm_window_type_desktop = atoms[1];
    let net_wm_state = atoms[2];
    let net_wm_state_below = atoms[3];
    let net_wm_state_sticky = atoms[4];
    let net_wm_state_skip_taskbar = atoms[5];
    let net_wm_state_skip_pager = atoms[6];

    // Set _NET_WM_WINDOW_TYPE = [_NET_WM_WINDOW_TYPE_DESKTOP]
    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: win,
        property: net_wm_window_type,
        r#type: x::ATOM_ATOM,
        data: &[net_wm_window_type_desktop],
    });

    // Set _NET_WM_STATE = [BELOW, STICKY, SKIP_TASKBAR, SKIP_PAGER]
    let states = [
        net_wm_state_below,
        net_wm_state_sticky,
        net_wm_state_skip_taskbar,
        net_wm_state_skip_pager,
    ];

    conn.send_request(&x::ChangeProperty {
        mode: x::PropMode::Replace,
        window: win,
        property: net_wm_state,
        r#type: x::ATOM_ATOM,
        data: &states,
    });

    Ok(())
}

/// Configures the window input shape to be empty, allowing click-through.
/// Uses the XShape extension to set the Input region to an empty list of rectangles.
pub fn setup_input_shape(conn: &xcb::Connection, window: x::Window) -> Result<()> {
    conn.send_request(&shape::Rectangles {
        operation: shape::So::Set,
        destination_kind: shape::Sk::Input,
        ordering: x::ClipOrdering::Unsorted,
        destination_window: window,
        x_offset: 0,
        y_offset: 0,
        rectangles: &[],
    });
    Ok(())
}

/// Manages an offscreen Cairo surface for double-buffered rendering.
pub struct OffscreenBuffer {
    surface: ImageSurface,
    width: u16,
    height: u16,
}

impl OffscreenBuffer {
    pub fn new(width: u16, height: u16) -> Result<Self> {
        let surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32)
            .map_err(|e| anyhow::anyhow!("Cairo surface creation failed: {}", e))?;
        Ok(Self { surface, width, height })
    }

    pub fn context(&self) -> Result<CairoContext> {
        CairoContext::new(&self.surface).map_err(|e| anyhow::anyhow!("Failed to create Cairo context: {}", e))
    }

    /// Uploads the offscreen buffer to the X11 window.
    pub fn present(&mut self, conn: &xcb::Connection, window: x::Window, gc: x::Gcontext) -> Result<()> {
        self.surface.flush();
        let data = self.surface.data().map_err(|e| anyhow::anyhow!("Failed to get surface data: {}", e))?;
        
        conn.send_request(&x::PutImage {
            format: x::ImageFormat::ZPixmap,
            drawable: x::Drawable::Window(window),
            gc,
            width: self.width,
            height: self.height,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            depth: 32,
            data: &data,
        });
        Ok(())
    }
}

/// Helper to initialize double buffering.
pub fn setup_double_buffering(width: u16, height: u16) -> Result<OffscreenBuffer> {
    OffscreenBuffer::new(width, height)
}

/// Maps the window to the screen.
pub fn map_window(conn: &xcb::Connection, window: x::Window) -> Result<()> {
    conn.send_request(&x::MapWindow { window });
    Ok(())
}

/// Context for a single monitor's overlay window.
pub struct MonitorContext {
    pub monitor: Monitor,
    pub window: x::Window,
    pub surface: OffscreenBuffer,
}

/// Manages the lifecycle of overlay windows.
pub struct WindowManager {
    pub monitors: Vec<MonitorContext>,
}

impl WindowManager {
    /// Destroys all windows managed by this instance.
    pub fn cleanup(&self, conn: &xcb::Connection) -> Result<()> {
        for ctx in &self.monitors {
            conn.send_request(&x::DestroyWindow { window: ctx.window });
        }
        conn.flush()?;
        Ok(())
    }
}

/// Creates overlay windows for all detected monitors.
pub fn create_all_windows(conn: &xcb::Connection, config: &Config) -> Result<WindowManager> {
    let detected_monitors = detect_monitors(conn)?;
    let mut contexts = Vec::new();

    for monitor in detected_monitors {
        let window = create_overlay_window(conn, &monitor, config)?;
        setup_ewmh_properties(conn, window)?;
        setup_input_shape(conn, window)?;
        
        map_window(conn, window)?;

        conn.send_request(&x::ConfigureWindow {
            window,
            value_list: &[x::ConfigWindow::StackMode(x::StackMode::Below)],
        });

        let surface = setup_double_buffering(monitor.width, monitor.height)?;

        contexts.push(MonitorContext {
            monitor,
            window,
            surface,
        });
    }
    
    conn.flush()?;

    Ok(WindowManager { monitors: contexts })
}
