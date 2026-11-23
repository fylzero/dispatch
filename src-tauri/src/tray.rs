//! The menu bar (macOS) / notification area (Windows, Linux) icon. It keeps
//! Dispatch reachable while the window is hidden, and Quit here always quits.
//!
//! Item ids start with "tray-": Tauri hands every menu event to the app
//! menu's handler as well, which forwards ids like "new-tab" to the page.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Wry};

use crate::settings::Settings;

pub const TRAY_ID: &str = "dispatch-tray";

fn item(app: &AppHandle, id: &str, label: &str, accelerator: &str) -> tauri::Result<MenuItem<Wry>> {
    let accelerator = (!accelerator.is_empty()).then_some(accelerator);
    /* A combo the menu cannot display should not take the whole menu down. */
    MenuItem::with_id(app, id, label, true, accelerator)
        .or_else(|_| MenuItem::with_id(app, id, label, true, None::<&str>))
}

fn menu(app: &AppHandle, settings: &Settings) -> tauri::Result<Menu<Wry>> {
    Menu::with_items(
        app,
        &[
            &item(
                app,
                "tray-toggle",
                "Show or Hide Dispatch",
                &settings.toggle_hotkey,
            )?,
            &item(app, "tray-new-tab", "New Tab", "")?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, "tray-settings", "Settings…", "")?,
            &item(app, "tray-updates", "Check for Updates…", "")?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, "tray-quit", "Quit Dispatch", "")?,
        ],
    )
}

fn icon() -> tauri::Result<Image<'static>> {
    /*
     * macOS draws a black-and-alpha template image in whatever color the menu
     * bar needs. Windows and Linux trays do not, so they get a full-color mark
     * that reads on light and dark taskbars.
     */
    if cfg!(target_os = "macos") {
        Image::from_bytes(include_bytes!("../icons/tray-template@2x.png"))
    } else {
        Image::from_bytes(include_bytes!("../icons/tray-color.png"))
    }
}

pub fn build(app: &AppHandle, settings: &Settings) -> tauri::Result<TrayIcon> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon()?)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Dispatch")
        .menu(&menu(app, settings)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-toggle" => crate::toggle_main(app),
            "tray-new-tab" => {
                crate::show_main(app);
                let _ = app.emit_to("main", "new-tab", ());
            }
            "tray-settings" => {
                crate::show_main(app);
                let _ = app.emit_to("main", "open-settings", ());
            }
            "tray-updates" => {
                crate::show_main(app);
                let _ = app.emit_to("main", "check-for-updates", ());
            }
            "tray-quit" => app.exit(0),
            _ => {}
        })
        .build(app)
}

/// Show the new combo next to Show or Hide Dispatch.
pub fn refresh(app: &AppHandle, settings: &Settings) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), menu(app, settings)) {
        let _ = tray.set_menu(Some(menu));
    }
}
