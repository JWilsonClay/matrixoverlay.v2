use tray_icon::{Icon, TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}, TrayIconEvent};
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    gtk::init()?;

    let menu = Menu::new();
    menu.append(&MenuItem::new("Test Item", true, None))?;
    menu.append(&MenuItem::with_id("quit", "Quit", true, None))?;

    let icon = generate_dummy_icon();
    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu.clone()))
        .with_tooltip("Tray Test")
        .with_icon(icon)
        .build()?;

    println!("Tray icon created. Click it! (Ctrl+C to stop)");

    let tray_channel = TrayIconEvent::receiver();
    let menu_channel = MenuEvent::receiver();

    loop {
        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration();
        }

        if let Ok(event) = tray_channel.try_recv() {
            println!("TRAY EVENT: {:?}", event);
        }

        if let Ok(event) = menu_channel.try_recv() {
            println!("MENU EVENT: {:?}", event);
            if event.id.as_ref() == "quit" {
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}

fn generate_dummy_icon() -> Icon {
    let width = 32;
    let height = 32;
    let rgba = vec![0, 255, 0, 255].repeat(width * height);
    Icon::from_rgba(rgba, width as u32, height as u32).unwrap()
}
