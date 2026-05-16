//! Sovereign System Tray Substrate.
//! Orchestrates the background presence and quick-access menu interactions.

use anyhow::Result;
use tray_icon::{Icon, TrayIconBuilder, menu::{Menu, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem}};
use crate::core::config::Config;
use notify_rust::Notification;

pub const MENU_QUIT_ID: &str = "quit";
pub const MENU_RELOAD_ID: &str = "reload";
pub const MENU_THEME_CLASSIC: &str = "theme_classic";
pub const MENU_THEME_CALM: &str = "theme_calm";
pub const MENU_THEME_ALERT: &str = "theme_alert";
pub const MENU_TOGGLE_AUTO_COMMIT: &str = "toggle_auto_commit";
pub const MENU_TOGGLE_OLLAMA: &str = "toggle_ollama";
pub const MENU_CONFIG_GUI_ID: &str = "config_gui";
pub const MENU_CONFIG_JSON_ID: &str = "config_json";

pub struct SystemTray { _tray: tray_icon::TrayIcon, _menu: Menu }

impl SystemTray {
    /// [HARDENED] Initializes the system tray with a failure-isolated menu hierarchy.
    pub fn new(config: &Config) -> Result<Self> {
        let icon = generate_icon()?;
        let menu = Menu::new();
        
        let config_submenu = Submenu::new("Settings / Config", true);
        config_submenu.append(&MenuItem::with_id(MENU_CONFIG_GUI_ID, "Open GUI Control Panel", true, None))?;
        config_submenu.append(&MenuItem::with_id(MENU_CONFIG_JSON_ID, "Edit JSON (IDE)", true, None))?;
        menu.append(&config_submenu)?;
        
        menu.append(&MenuItem::with_id(MENU_RELOAD_ID, "Reload Overlay", true, None))?;
        menu.append(&PredefinedMenuItem::separator())?;
        
        let theme_submenu = Submenu::new("Themes", true);
        theme_submenu.append(&MenuItem::with_id(MENU_THEME_CLASSIC, "Classic Green", true, None))?;
        theme_submenu.append(&MenuItem::with_id(MENU_THEME_CALM, "Calm Blue", true, None))?;
        theme_submenu.append(&MenuItem::with_id(MENU_THEME_ALERT, "Alert Red", true, None))?;
        menu.append(&theme_submenu)?;
        
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&CheckMenuItem::with_id(MENU_TOGGLE_AUTO_COMMIT, "Auto-Commit Status", true, config.productivity.auto_commit_threshold > 0, None))?;
        menu.append(&CheckMenuItem::with_id(MENU_TOGGLE_OLLAMA, "Ollama AI Insights", true, config.productivity.ollama_enabled, None))?;
        
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&MenuItem::with_id(MENU_QUIT_ID, "Quit", true, None))?;
        
        let tray = TrayIconBuilder::new().with_menu(Box::new(menu.clone())).with_tooltip("Matrix Overlay v2").with_icon(icon).build()?;
        Ok(Self { _tray: tray, _menu: menu })
    }

    /// [HARDENED] One-time update notification with failure-isolated delivery.
    pub fn show_update_bubble(version: &str) {
        let _ = Notification::new()
            .summary("Matrix Overlay - Update Available")
            .body(&format!("A new version (v{}) is ready for secure delivery.", version))
            .icon("system-software-update").timeout(10000).show();
    }
}

fn generate_icon() -> Result<Icon> {
    let (width, height) = (32, 32);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) { rgba.extend_from_slice(&[0, 255, 65, 255]); }
    Icon::from_rgba(rgba, width, height).map_err(|e| anyhow::anyhow!("Icon fail: {}", e))
}
