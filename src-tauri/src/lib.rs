mod blur;
mod corners;
mod hotkey;
mod menu;
mod pty;
mod settings;
mod tray;

use pty::TerminalState;
use settings::SettingsState;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WindowEvent};

/// "Keep running in the background": close and ⌘Q hide the window instead.
pub fn keep_running(app: &AppHandle) -> bool {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|settings| settings.keep_running_in_background)
        .unwrap_or(false)
}

/// Bring the window forward on the desktop the user is on.
pub fn show_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    /*
     * Hidden, Dispatch lives in the menu bar only. An unbundled dev binary
     * keeps its Dock icon throughout: switching it back and forth loses the
     * app's name and icon.
     */
    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// Back to the menu bar. The tabs and their shells keep running.
pub fn hide_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    #[cfg(target_os = "macos")]
    {
        #[cfg(not(debug_assertions))]
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        /* Hand focus back to the app the user came from. */
        let _ = app.hide();
    }
}

/// The toggle hotkey: hide Dispatch if it is in front, otherwise bring it up.
pub fn toggle_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false);
    if visible && window.is_focused().unwrap_or(false) {
        hide_main(app);
    } else {
        show_main(app);
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    hide_main(&app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        /* Opening Dispatch again while it runs in the background shows it. */
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(hotkey::handle)
                .build(),
        )
        .menu(menu::build)
        .on_menu_event(|app, event| {
            menu::handle_event(app, event.id().as_ref());
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let loaded = settings::load(&handle);
            app.manage(hotkey::HotkeyState::default());
            if let Err(err) = hotkey::register_initial(&handle, &loaded.toggle_hotkey) {
                eprintln!("{err}");
            }
            tray::build(&handle, &loaded)?;
            app.manage(SettingsState(Mutex::new(loaded)));
            app.manage(TerminalState::default());
            if let Some(window) = app.get_webview_window("main") {
                corners::round(&window);
                /* The hotkey opens the window on the Space the user is on, not the one it was left on. */
                follow_active_space(&window);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if keep_running(window.app_handle()) {
                    api.prevent_close();
                    hide_main(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::save_settings,
            settings::settings_file_path,
            blur::set_window_blur,
            pty::open_terminal,
            pty::write_terminal,
            pty::resize_terminal,
            pty::close_terminal,
            hotkey::set_toggle_hotkey,
            hotkey::pause_toggle_hotkey,
            hide_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Dispatch")
        .run(|app, event| match event {
            /* With the window hidden, Dispatch keeps running in the menu bar. */
            tauri::RunEvent::ExitRequested { api, code, .. } if code.is_none() && keep_running(app) => {
                api.prevent_exit();
            }
            /* Clicking the Dock icon (or opening the app) while the window is hidden. */
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show_main(app),
            _ => {}
        });
}

#[cfg(target_os = "macos")]
fn follow_active_space(window: &tauri::WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    /// NSWindowCollectionBehaviorMoveToActiveSpace
    const MOVE_TO_ACTIVE_SPACE: usize = 1 << 1;
    let Ok(ns_window) = window.ns_window() else {
        return;
    };
    let ns_window = ns_window as *mut AnyObject;
    if ns_window.is_null() {
        return;
    }
    unsafe {
        let behavior: usize = msg_send![ns_window, collectionBehavior];
        let _: () = msg_send![ns_window, setCollectionBehavior: behavior | MOVE_TO_ACTIVE_SPACE];
    }
}

#[cfg(not(target_os = "macos"))]
fn follow_active_space(_window: &tauri::WebviewWindow) {}
