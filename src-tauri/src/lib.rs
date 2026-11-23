mod blur;
mod menu;
mod pty;
mod settings;

use pty::TerminalState;
use settings::SettingsState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .menu(menu::build)
        .on_menu_event(|app, event| {
            menu::handle_event(app, event.id().as_ref());
        })
        .setup(|app| {
            let loaded = settings::load(app.handle());
            app.manage(SettingsState(Mutex::new(loaded)));
            app.manage(TerminalState::default());
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
