use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

use crate::aliases::AliasGroup;

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
    /// How much unfocused panes in a split tab are dimmed, from 0 (off) to 0.8.
    pub pane_dim: f32,
    /// How a named pane shows its name: "notch", "bar" or "clear".
    pub pane_label_style: String,
    /// Where the pane name sits along the top edge: "left", "center" or "right".
    pub pane_label_align: String,
    /// Replace the zsh or bash prompt with chips for the Node version, folder,
    /// git branch, and changed files. Applies to new tabs.
    pub dispatch_prompt: bool,
    /// With the Dispatch prompt, commands are typed on the line below the
    /// chips instead of right after them. Applies to new tabs.
    pub type_below_prompt: bool,
    /// As you type, the file, folder, or command Tab would most likely
    /// complete shows in grey after the cursor. PowerShell and cmd get the
    /// marks it needs from new tabs on; zsh and bash from the Dispatch prompt.
    pub suggestions: bool,
    /// The key that fills in a suggestion: "tab", "right" (Right arrow, End
    /// and Command+Right), or "both" (the default).
    pub suggestion_accept_key: String,
    /// Short names for commands, in named groups, defined in every new tab
    /// after the user's own startup files (see aliases.rs).
    pub alias_groups: Vec<AliasGroup>,
    /// Closing the window or quitting with ⌘Q hides Dispatch and keeps its
    /// tabs running, so the toggle hotkey can bring them straight back.
    /// Quit from the menu bar icon still quits.
    pub keep_running_in_background: bool,
    /// Global shortcut that shows or hides Dispatch from any app. Empty is off.
    pub toggle_hotkey: String,
    /// Look for a new release at launch and every few hours, and say when one
    /// is ready to install.
    pub auto_check_updates: bool,
    /// Layouts saved from the tab bar's menu: each a name and its tabs, with
    /// their split panes, names, colors and sizes. Kept as plain JSON, since
    /// only the window reads them.
    pub layouts: Vec<serde_json::Value>,
    /// The saved layout the first window opens with when Dispatch starts.
    /// Later windows, and blank, open one plain tab.
    pub default_layout: String,
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
            font_size: 15.0,
            line_height: 1.2,
            cursor_style: "bar".into(),
            cursor_blink: true,
            shell: String::new(),
            start_directory: "~".into(),
            scrollback: 10_000,
            opacity: 0.8,
            blur: 8,
            pane_dim: 0.4,
            pane_label_style: "notch".into(),
            pane_label_align: "left".into(),
            dispatch_prompt: true,
            type_below_prompt: true,
            suggestions: true,
            suggestion_accept_key: "both".into(),
            alias_groups: Vec::new(),
            keep_running_in_background: true,
            toggle_hotkey: default_toggle_hotkey(),
            auto_check_updates: true,
            layouts: Vec::new(),
            default_layout: String::new(),
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
    window: WebviewWindow,
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
    drop(current);
    crate::alias_files::refresh(&crate::aliases::flatten(&settings.alias_groups));
    /* The other windows follow along. */
    for label in app.webview_windows().into_keys() {
        if label != window.label() {
            let _ = app.emit_to(label.as_str(), "settings-changed", &settings);
        }
    }
    Ok(settings)
}

#[tauri::command]
pub fn settings_file_path(app: AppHandle) -> Result<String, String> {
    Ok(settings_path(&app)?.to_string_lossy().into_owned())
}

/// Writes a settings export to the file the user picked.
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Choose a file to export.".into());
    }
    fs::write(&path, contents).map_err(|err| format!("Could not write the file: {err}"))
}

/// Reads a settings file the user picked to import.
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("Choose a file to import.".into());
    }
    fs::read_to_string(&path).map_err(|err| format!("Could not read the file: {err}"))
}
