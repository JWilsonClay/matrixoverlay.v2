// src/tray.rs
use anyhow::Result;
use tray_icon::{Icon, TrayIconBuilder, menu::{Menu, MenuItem}};

pub const MENU_QUIT_ID: &str = "quit";
pub const MENU_RELOAD_ID: &str = "reload";
pub const MENU_EDIT_ID: &str = "edit";

pub struct SystemTray {
    _tray: tray_icon::TrayIcon,
}

impl SystemTray {
    pub fn new() -> Result<Self> {
        let icon = generate_icon()?;
        let menu = Menu::new();
        
        // Settings Submenu
        let settings_menu = Menu::new();
        let edit_item = MenuItem::with_id(MENU_EDIT_ID, "Edit Config", true, None);
        let reload_item = MenuItem::with_id(MENU_RELOAD_ID, "Reload Config", true, None);
        settings_menu.append(&edit_item)?;
        settings_menu.append(&reload_item)?;
        
        let settings_submenu = tray_icon::menu::Submenu::new("Settings", true);
        settings_submenu.append(&edit_item)?;
        settings_submenu.append(&reload_item)?;
        
        let about_item = MenuItem::with_id("about", "About Matrix v2", true, None);
        let quit_item = MenuItem::with_id(MENU_QUIT_ID, "Quit", true, None);
        
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
