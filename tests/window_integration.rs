//! Integration tests for Window Management.
//! Verifies X11 atoms, layering, input shapes, and geometry.
//!
//! Note: These tests require an active X11 server (DISPLAY set).
//! They will gracefully skip if connection fails (e.g. in headless CI without Xvfb).

use xcb::x;
use xcb::shape;
use xcb::Xid;

use matrix_overlay::config::Config;
use matrix_overlay::window::create_all_windows;

/// Helper to setup X11 connection for tests.
/// Returns None if X server is unavailable.
fn setup_x11() -> Option<(xcb::Connection, i32)> {
    match xcb::Connection::connect(None) {
        Ok((conn, screen)) => Some((conn, screen)),
        Err(e) => {
            eprintln!("Skipping integration test (X11 connection failed): {}", e);
            None
        }
    }
}

#[test]
fn test_window_properties_and_atoms() {
    let (conn, _screen_num) = match setup_x11() {
        Some(v) => v,
        None => return,
    };

    let config = Config::default();
    // Initialize WindowManager (creates windows)
    let wm = create_all_windows(&conn, &config)
        .expect("Failed to create windows");

    if wm.monitors.is_empty() {
        eprintln!("No monitors detected/windows created. Skipping assertions.");
        return;
    }

    for monitor in &wm.monitors {
        let win = monitor.window;

        // Intern atoms manually for verification
        let net_wm_window_type = conn.send_request(&x::InternAtom { only_if_exists: true, name: b"_NET_WM_WINDOW_TYPE" });
        let net_wm_window_type_desktop = conn.send_request(&x::InternAtom { only_if_exists: true, name: b"_NET_WM_WINDOW_TYPE_DESKTOP" });
        let net_wm_state = conn.send_request(&x::InternAtom { only_if_exists: true, name: b"_NET_WM_STATE" });
        let net_wm_state_below = conn.send_request(&x::InternAtom { only_if_exists: true, name: b"_NET_WM_STATE_BELOW" });
        let net_wm_state_skip_taskbar = conn.send_request(&x::InternAtom { only_if_exists: true, name: b"_NET_WM_STATE_SKIP_TASKBAR" });
        let net_wm_state_skip_pager = conn.send_request(&x::InternAtom { only_if_exists: true, name: b"_NET_WM_STATE_SKIP_PAGER" });

        let net_wm_window_type = conn.wait_for_reply(net_wm_window_type).unwrap().atom();
        let net_wm_window_type_desktop = conn.wait_for_reply(net_wm_window_type_desktop).unwrap().atom();
        let net_wm_state = conn.wait_for_reply(net_wm_state).unwrap().atom();
        let net_wm_state_below = conn.wait_for_reply(net_wm_state_below).unwrap().atom();
        let net_wm_state_skip_taskbar = conn.wait_for_reply(net_wm_state_skip_taskbar).unwrap().atom();
        let net_wm_state_skip_pager = conn.wait_for_reply(net_wm_state_skip_pager).unwrap().atom();

        // 1. Verify _NET_WM_WINDOW_TYPE is _NET_WM_WINDOW_TYPE_DESKTOP
        let cookie = conn.send_request(&x::GetProperty {
            delete: false,
            window: win,
            property: net_wm_window_type,
            r#type: x::ATOM_ATOM,
            long_offset: 0,
            long_length: 1024,
        });
        let reply = conn.wait_for_reply(cookie).unwrap();

        assert_eq!(reply.format(), 32, "Property format should be 32-bit");
        let types: Vec<x::Atom> = reply.value::<x::Atom>().into();
        assert!(
            types.contains(&net_wm_window_type_desktop),
            "Window {:x} missing _NET_WM_WINDOW_TYPE_DESKTOP", win.resource_id()
        );

        // 2. Verify _NET_WM_STATE contains BELOW, SKIP_TASKBAR, SKIP_PAGER
        let cookie = conn.send_request(&x::GetProperty {
            delete: false,
            window: win,
            property: net_wm_state,
            r#type: x::ATOM_ATOM,
            long_offset: 0,
            long_length: 1024,
        });
        let reply = conn.wait_for_reply(cookie).unwrap();

        let states: Vec<x::Atom> = reply.value::<x::Atom>().into();
        assert!(states.contains(&net_wm_state_below), "Missing _NET_WM_STATE_BELOW");
        assert!(states.contains(&net_wm_state_skip_taskbar), "Missing _NET_WM_STATE_SKIP_TASKBAR");
        assert!(states.contains(&net_wm_state_skip_pager), "Missing _NET_WM_STATE_SKIP_PAGER");
    }
}

#[test]
fn test_click_through_input_shape() {
    let (conn, _screen_num) = match setup_x11() {
        Some(v) => v,
        None => return,
    };

    let config = Config::default();
    let wm = create_all_windows(&conn, &config).unwrap();

    for monitor in &wm.monitors {
        let win = monitor.window;

        // Query Input Shape Rectangles
        // We expect 0 rectangles, meaning the input region is empty (passthrough)
        let cookie = conn.send_request(&shape::GetRectangles {
            window: win,
            source_kind: shape::Sk::Input,
        });
        let reply = conn.wait_for_reply(cookie).unwrap();
        
        assert_eq!(
            reply.rectangles().len(), 0,
            "Window {:?} input shape is not empty (rects: {}). Click-through failed.",
            win, reply.rectangles().len()
        );
    }
}

#[test]
fn test_geometry_and_visual() {
    let (conn, _screen_num) = match setup_x11() {
        Some(v) => v,
        None => return,
    };

    let config = Config::default();
    let wm = create_all_windows(&conn, &config).unwrap();

    for monitor in &wm.monitors {
        let cookie = conn.send_request(&x::GetGeometry { drawable: x::Drawable::Window(monitor.window) });
        let geom = conn.wait_for_reply(cookie).unwrap();

        // Verify Depth 32 (ARGB)
        assert_eq!(geom.depth(), 32, "Window depth must be 32 for transparency");

        // Verify dimensions match what the WM thinks (which is derived from RandR)
        assert_eq!(geom.width(), monitor.monitor.width);
        assert_eq!(geom.height(), monitor.monitor.height);
        
        // Verify position
        // Note: Window creation applies offsets from config.
        // Since we use Config::default(), offsets are 20, 20.
        let (off_x, off_y) = config.screens.first()
            .map(|s| (s.x_offset, s.y_offset))
            .unwrap_or((0, 0));

        assert_eq!(geom.x(), (monitor.monitor.x as i32 + off_x) as i16, "Window X position mismatch (check offsets)");
        assert_eq!(geom.y(), (monitor.monitor.y as i32 + off_y) as i16, "Window Y position mismatch (check offsets)");
    }
}