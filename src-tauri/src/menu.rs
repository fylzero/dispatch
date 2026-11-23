use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Menu items whose id is also the event the page listens for.
pub const NEW_TAB_ID: &str = "new-tab";
pub const CLOSE_TAB_OR_WINDOW_ID: &str = "close-tab-or-window";
pub const NEXT_TAB_ID: &str = "next-tab";
pub const PREVIOUS_TAB_ID: &str = "previous-tab";
pub const OPEN_SETTINGS_ID: &str = "open-settings";
pub const CLEAR_ID: &str = "clear-terminal";
pub const ZOOM_IN_ID: &str = "zoom-in";
pub const ZOOM_OUT_ID: &str = "zoom-out";
pub const ZOOM_RESET_ID: &str = "zoom-reset";
pub const SELECT_ALL_ID: &str = "select-all";
pub const CHECK_FOR_UPDATES_ID: &str = "check-for-updates";
pub const SPLIT_VERTICAL_ID: &str = "split-vertical";
pub const SPLIT_HORIZONTAL_ID: &str = "split-horizontal";
pub const FOCUS_LEFT_ID: &str = "focus-pane-left";
pub const FOCUS_RIGHT_ID: &str = "focus-pane-right";
pub const FOCUS_UP_ID: &str = "focus-pane-up";
pub const FOCUS_DOWN_ID: &str = "focus-pane-down";
/// Not "quit" or the predefined Quit: that one sends terminate: straight to
/// NSApp, ending the process before "Keep running in the background" can
/// turn ⌘Q into hiding the window.
pub const QUIT_ID: &str = "app-quit";

/// On macOS terminal shortcuts use Command; elsewhere Ctrl+W, Ctrl+T, Ctrl+K
/// and friends belong to the shell, so they take Ctrl+Shift instead.
#[cfg(target_os = "macos")]
const MOD: &str = "Cmd";
#[cfg(not(target_os = "macos"))]
const MOD: &str = "Ctrl+Shift";

/// Splits: ⌘D / ⌘⇧D like iTerm on macOS. Elsewhere Ctrl+Shift+D is taken by
/// the side-by-side split, so stacking uses Ctrl+Shift+E (as in Terminator).
#[cfg(target_os = "macos")]
const SPLIT_KEYS: (&str, &str) = ("Cmd+D", "Cmd+Shift+D");
#[cfg(not(target_os = "macos"))]
const SPLIT_KEYS: (&str, &str) = ("Ctrl+Shift+D", "Ctrl+Shift+E");

/// Moving between panes: ⌘⌥ + arrow on macOS, Alt+Shift + arrow elsewhere.
#[cfg(target_os = "macos")]
const PANE_MOD: &str = "Cmd+Alt";
#[cfg(not(target_os = "macos"))]
const PANE_MOD: &str = "Alt+Shift";

const FORWARDED: &[&str] = &[
    NEW_TAB_ID,
    CLOSE_TAB_OR_WINDOW_ID,
    NEXT_TAB_ID,
    PREVIOUS_TAB_ID,
    OPEN_SETTINGS_ID,
    CLEAR_ID,
    ZOOM_IN_ID,
    ZOOM_OUT_ID,
    ZOOM_RESET_ID,
    SELECT_ALL_ID,
    CHECK_FOR_UPDATES_ID,
    SPLIT_VERTICAL_ID,
    SPLIT_HORIZONTAL_ID,
    FOCUS_LEFT_ID,
    FOCUS_RIGHT_ID,
    FOCUS_UP_ID,
    FOCUS_DOWN_ID,
];

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let pkg_info = app.package_info();
    let config = app.config();
    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };

    let item =
        |id: &str, label: &str, accel: &str| MenuItem::with_id(app, id, label, true, Some(accel));

    let new_tab = item(NEW_TAB_ID, "New Tab", &format!("{MOD}+T"))?;
    let close = item(CLOSE_TAB_OR_WINDOW_ID, "Close", &format!("{MOD}+W"))?;
    let split_vertical = item(SPLIT_VERTICAL_ID, "Split Vertically", SPLIT_KEYS.0)?;
    let split_horizontal = item(SPLIT_HORIZONTAL_ID, "Split Horizontally", SPLIT_KEYS.1)?;
    let focus_left = item(
        FOCUS_LEFT_ID,
        "Select Pane on Left",
        &format!("{PANE_MOD}+Left"),
    )?;
    let focus_right = item(
        FOCUS_RIGHT_ID,
        "Select Pane on Right",
        &format!("{PANE_MOD}+Right"),
    )?;
    let focus_up = item(FOCUS_UP_ID, "Select Pane Above", &format!("{PANE_MOD}+Up"))?;
    let focus_down = item(
        FOCUS_DOWN_ID,
        "Select Pane Below",
        &format!("{PANE_MOD}+Down"),
    )?;
    let settings = item(OPEN_SETTINGS_ID, "Settings…", "CmdOrCtrl+,")?;
    #[cfg(target_os = "macos")]
    let quit = item(QUIT_ID, &format!("Quit {}", pkg_info.name), "Cmd+Q")?;
    let check_updates = MenuItem::with_id(
        app,
        CHECK_FOR_UPDATES_ID,
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let clear = item(CLEAR_ID, "Clear", &format!("{MOD}+K"))?;
    let select_all = item(SELECT_ALL_ID, "Select All", &format!("{MOD}+A"))?;
    let zoom_in = item(ZOOM_IN_ID, "Bigger", "CmdOrCtrl+=")?;
    let zoom_out = item(ZOOM_OUT_ID, "Smaller", "CmdOrCtrl+-")?;
    let zoom_reset = item(ZOOM_RESET_ID, "Actual Size", "CmdOrCtrl+0")?;
    let next_tab = item(NEXT_TAB_ID, "Show Next Tab", "CmdOrCtrl+Shift+]")?;
    let previous_tab = item(PREVIOUS_TAB_ID, "Show Previous Tab", "CmdOrCtrl+Shift+[")?;

    Menu::with_items(
        app,
        &[
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                pkg_info.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app, None, Some(about_metadata.clone()))?,
                    &check_updates,
                    &PredefinedMenuItem::separator(app)?,
                    &settings,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?,
            #[cfg(target_os = "macos")]
            &Submenu::with_items(
                app,
                "Shell",
                true,
                &[
                    &new_tab,
                    &PredefinedMenuItem::separator(app)?,
                    &split_vertical,
                    &split_horizontal,
                    &PredefinedMenuItem::separator(app)?,
                    &close,
                ],
            )?,
            #[cfg(not(target_os = "macos"))]
            &Submenu::with_items(
                app,
                "File",
                true,
                &[
                    &new_tab,
                    &split_vertical,
                    &split_horizontal,
                    &PredefinedMenuItem::separator(app)?,
                    &settings,
                    &close,
                ],
            )?,
            /* Native Copy and Paste reach xterm's own copy/paste handlers. On
             * Windows and Linux the page handles Ctrl+Shift+C/V instead, so
             * Ctrl+C stays an interrupt. */
            &Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    #[cfg(target_os = "macos")]
                    &PredefinedMenuItem::copy(app, None)?,
                    #[cfg(target_os = "macos")]
                    &PredefinedMenuItem::paste(app, None)?,
                    &select_all,
                    &PredefinedMenuItem::separator(app)?,
                    &clear,
                ],
            )?,
            &Submenu::with_items(
                app,
                "View",
                true,
                &[
                    &zoom_reset,
                    &zoom_in,
                    &zoom_out,
                    #[cfg(target_os = "macos")]
                    &PredefinedMenuItem::separator(app)?,
                    #[cfg(target_os = "macos")]
                    &PredefinedMenuItem::fullscreen(app, None)?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, None)?,
                    &PredefinedMenuItem::maximize(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &next_tab,
                    &previous_tab,
                    &PredefinedMenuItem::separator(app)?,
                    &focus_left,
                    &focus_right,
                    &focus_up,
                    &focus_down,
                ],
            )?,
            #[cfg(not(target_os = "macos"))]
            &Submenu::with_items(
                app,
                "Help",
                true,
                &[
                    &PredefinedMenuItem::about(app, None, Some(about_metadata))?,
                    &check_updates,
                ],
            )?,
        ],
    )
}

pub fn handle_event(app: &AppHandle, id: &str) {
    if id == QUIT_ID {
        if crate::keep_running(app) {
            crate::hide_main(app);
        } else {
            app.exit(0);
        }
        return;
    }
    if !FORWARDED.contains(&id) {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(id, ());
    }
}
