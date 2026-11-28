mod blur;
mod corners;
mod hotkey;
mod menu;
mod prompt;
mod pty;
mod settings;
mod suggest;
mod tray;
mod windows;

use pty::TerminalState;
use settings::SettingsState;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WindowEvent};
use windows::WindowsState;

/// "Keep running in the background": close and ⌘Q hide the window instead.
pub fn keep_running(app: &AppHandle) -> bool {
    app.state::<SettingsState>()
        .0
        .lock()
        .map(|settings| settings.keep_running_in_background)
        .unwrap_or(false)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        /* Opening Dispatch again while it runs in the background shows it. */
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::show(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        /* Only the first window's size and place are remembered; ⌘N windows cascade from the current one. */
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_filter(|label| label == windows::MAIN)
                .build(),
        )
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
            app.manage(WindowsState::default());
            if let Some(window) = app.get_webview_window(windows::MAIN) {
                windows::prepare(&window);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::Focused(true) => windows::remember_focus(app, window.label()),
                /* Closing the last window hides it instead, so its tabs are there next time. */
                WindowEvent::CloseRequested { api, .. } => {
                    if let Some(webview) = app.get_webview_window(window.label()) {
                        if windows::is_last(&webview) && keep_running(app) {
                            api.prevent_close();
                            windows::hide(app);
                        }
                    }
                }
                /* A closed window's shells go with it. */
                WindowEvent::Destroyed => pty::close_window_terminals(app, window.label()),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            settings::get_settings,
            settings::save_settings,
            settings::settings_file_path,
            settings::read_text_file,
            settings::write_text_file,
            blur::set_window_blur,
            pty::open_terminal,
            pty::write_terminal,
            pty::resize_terminal,
            pty::close_terminal,
            pty::terminal_cwd,
            suggest::suggest_completion,
            hotkey::set_toggle_hotkey,
            hotkey::pause_toggle_hotkey,
            windows::close_window,
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
            tauri::RunEvent::Reopen { .. } => windows::show(app),
            _ => {}
        });
}
