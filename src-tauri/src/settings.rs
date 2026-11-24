use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: String,
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub cursor_style: String,
    pub cursor_blink: bool,
    /// Blank means the login shell from $SHELL.
    pub shell: String,
    /// Folder new tabs start in. `~` is the home folder, and so is a blank
    /// value or one that is not a folder.
    pub start_directory: String,
    pub scrollback: u32,
    /// How opaque the terminal background is, from 0.3 to 1.
    pub opacity: f32,
    /// Blur radius for what shows through the background; 0 turns it off.
    pub blur: u32,
    /// Replace the zsh or bash prompt with chips for the Node version, folder,
    /// git branch, and changed files. Applies to new tabs.
    pub dispatch_prompt: bool,
    /// Closing the window or quitting with ⌘Q hides Dispatch and keeps its
    /// tabs running, so the toggle hotkey can bring them straight back.
    /// Quit from the menu bar icon still quits.
    pub keep_running_in_background: bool,
    /// Global shortcut that shows or hides Dispatch from any app. Empty is off.
    pub toggle_hotkey: String,
}

/// ⌘` on macOS, Ctrl+` elsewhere.
pub fn default_toggle_hotkey() -> String {
    if cfg!(target_os = "macos") {
        "Cmd+Backquote".into()
    } else {
        "Ctrl+Backquote".into()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            font_family: "jetbrains".into(),
            font_size: 14.0,
            line_height: 1.3,
            cursor_style: "block".into(),
            cursor_blink: true,
            shell: String::new(),
            start_directory: "~".into(),
            scrollback: 10_000,
            opacity: 0.8,
            blur: 12,
            dispatch_prompt: true,
            keep_running_in_background: true,
            toggle_hotkey: default_toggle_hotkey(),
        }
    }
}

pub struct SettingsState(pub Mutex<Settings>);

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Could not resolve the app data directory: {err}"))?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not create the app data directory: {err}"))?;
    Ok(dir.join(SETTINGS_FILE))
}

pub fn load(app: &AppHandle) -> Settings {
    settings_path(app)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_settings(state: State<SettingsState>) -> Result<Settings, String> {
    state
        .0
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "Could not read settings.".into())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<SettingsState>,
    settings: Settings,
) -> Result<Settings, String> {
    let raw = serde_json::to_string_pretty(&settings)
        .map_err(|err| format!("Could not serialize {SETTINGS_FILE}: {err}"))?;
    fs::write(settings_path(&app)?, raw)
        .map_err(|err| format!("Could not write {SETTINGS_FILE}: {err}"))?;
    let mut current = state
        .0
        .lock()
        .map_err(|_| "Could not update settings.".to_string())?;
    *current = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn settings_file_path(app: AppHandle) -> Result<String, String> {
    Ok(settings_path(&app)?.to_string_lossy().into_owned())
}
