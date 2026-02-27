// src/tray.rs
use anyhow::Result;
use tray_icon::{Icon, TrayIconBuilder, menu::{Menu, MenuItem}};

pub const MENU_QUIT_ID: &str = "quit";
pub const MENU_RELOAD_ID: &str = "reload";
pub const MENU_EDIT_ID: &str = "edit";
pub const MENU_THEME_CLASSIC: &str = "theme_classic";
pub const MENU_THEME_CALM: &str = "theme_calm";
pub const MENU_THEME_ALERT: &str = "theme_alert";
pub const MENU_TOGGLE_AUTO_COMMIT: &str = "toggle_auto_commit";
pub const MENU_TOGGLE_OLLAMA: &str = "toggle_ollama";

pub struct SystemTray {
    _tray: tray_icon::TrayIcon,
}

impl SystemTray {
    pub fn new() -> Result<Self> {
        let icon = generate_icon()?;
        let menu = Menu::new();
        
        // Theme Submenu
        let theme_menu = Menu::new();
        theme_menu.append(&MenuItem::with_id(MENU_THEME_CLASSIC, "Classic Green", true, None))?;
        theme_menu.append(&MenuItem::with_id(MENU_THEME_CALM, "Calm Cyan", true, None))?;
        theme_menu.append(&MenuItem::with_id(MENU_THEME_ALERT, "Alert Red", true, None))?;
        let theme_submenu = tray_icon::menu::Submenu::new("Themes", true);
        theme_submenu.append(&MenuItem::with_id(MENU_THEME_CLASSIC, "Classic Green", true, None))?;
        theme_submenu.append(&MenuItem::with_id(MENU_THEME_CALM, "Calm Cyan", true, None))?;
        theme_submenu.append(&MenuItem::with_id(MENU_THEME_ALERT, "Alert Red", true, None))?;

        // Productivity Submenu (Toggles)
        let prod_submenu = tray_icon::menu::Submenu::new("Productivity", true);
        prod_submenu.append(&MenuItem::with_id(MENU_TOGGLE_AUTO_COMMIT, "Auto-Commits", true, None))?;
        prod_submenu.append(&MenuItem::with_id(MENU_TOGGLE_OLLAMA, "AI Summaries (Ollama)", true, None))?;

        // Settings Submenu
        let settings_submenu = tray_icon::menu::Submenu::new("Settings", true);
        settings_submenu.append(&MenuItem::with_id(MENU_EDIT_ID, "Edit Config", true, None))?;
        settings_submenu.append(&MenuItem::with_id(MENU_RELOAD_ID, "Reload Config", true, None))?;
        
        let about_item = MenuItem::with_id("about", "About Matrix v2", true, None);
        let quit_item = MenuItem::with_id(MENU_QUIT_ID, "Quit", true, None);
        
        menu.append(&theme_submenu)?;
        menu.append(&prod_submenu)?;
        menu.append(&settings_submenu)?;
        menu.append(&about_item)?;
        menu.append(&quit_item)?;
        
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("X11 Monitor Overlay")
            .with_icon(icon)
            .build()?;

        Ok(Self { _tray: tray })
    }
}

fn generate_icon() -> Result<Icon> {
    // Generate a simple 32x32 green square
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        // Matrix Green: R=0, G=255, B=65, A=255
        rgba.extend_from_slice(&[0, 255, 65, 255]);
    }
    Icon::from_rgba(rgba, width, height).map_err(|e| anyhow::anyhow!("Failed to create icon: {}", e))
}
