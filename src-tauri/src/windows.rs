//! Dispatch can have several windows (⌘N), each a page with its own tabs.
//! The first is "main", from tauri.conf.json; the rest are "window-1",
//! "window-2" and so on, built from the same config.
//!
//! Shown and hidden, they move together: the toggle hotkey, the menu bar icon
//! and "Keep running in the background" treat them as one app. Menu shortcuts
//! go to the window in front, or the one that was in front last.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WebviewWindowBuilder};

pub const MAIN: &str = "main";
const PREFIX: &str = "window-";
/// How far a new window sits down and to the right of the one it came from.
const CASCADE: f64 = 28.0;

#[derive(Default)]
pub struct WindowsState {
    /// The window that had focus last, for when none has it now (hidden, or
    /// another app in front).
    last_focused: Mutex<Option<String>>,
    next: AtomicUsize,
}

pub fn remember_focus(app: &AppHandle, label: &str) {
    if let Ok(mut last) = app.state::<WindowsState>().last_focused.lock() {
        *last = Some(label.to_string());
    }
}

fn all(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows().into_values().collect()
}

/// The window in front, else the one that was in front last, else any.
pub fn current(app: &AppHandle) -> Option<WebviewWindow> {
    let windows = app.webview_windows();
    if let Some(window) = windows.values().find(|w| w.is_focused().unwrap_or(false)) {
        return Some(window.clone());
    }
    let last = app
        .state::<WindowsState>()
        .last_focused
        .lock()
        .ok()
        .and_then(|last| last.clone());
    last.and_then(|label| windows.get(&label).cloned())
        .or_else(|| windows.get(MAIN).cloned())
        .or_else(|| windows.into_values().next())
}

/// Sends a menu or tray event to the current window's page only.
pub fn emit_to_current(app: &AppHandle, event: &str) {
    if let Some(window) = current(app) {
        let _ = app.emit_to(window.label(), event, ());
    }
}

/// Whether `window` is the only one left.
pub fn is_last(window: &WebviewWindow) -> bool {
    window
        .app_handle()
        .webview_windows()
        .keys()
        .all(|label| label == window.label())
}

/// Bring every window forward on the desktop the user is on, the current one
/// in front.
pub fn show(app: &AppHandle) {
    let windows = all(app);
    if windows.is_empty() {
        let _ = open(app);
        return;
    }
    /*
     * Hidden, Dispatch lives in the menu bar only. An unbundled dev binary
     * keeps its Dock icon throughout: switching it back and forth loses the
     * app's name and icon.
     */
    #[cfg(all(target_os = "macos", not(debug_assertions)))]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    #[cfg(target_os = "macos")]
    let _ = app.show();
    let front = current(app);
    for window in &windows {
        let _ = window.show();
    }
    if let Some(window) = front {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Back to the menu bar. The windows, their tabs and their shells keep running.
pub fn hide(app: &AppHandle) {
    for window in all(app) {
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
pub fn toggle(app: &AppHandle) {
    if all(app).iter().any(|w| w.is_focused().unwrap_or(false)) {
        hide(app);
    } else {
        show(app);
    }
}

/// What every window needs once it exists, "main" included.
pub fn prepare(window: &WebviewWindow) {
    crate::corners::round(window);
    /* The hotkey opens the window on the Space the user is on, not the one it was left on. */
    follow_active_space(window);
}

/// A new window with one fresh tab, the size of the current window and a
/// little down and to the right of it.
pub fn open(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let from = current(app);
    let mut config = app
        .config()
        .app
        .windows
        .first()
        .cloned()
        .unwrap_or_default();
    let n = app.state::<WindowsState>().next.fetch_add(1, Ordering::Relaxed) + 1;
    config.label = format!("{PREFIX}{n}");
    config.center = false;
    if let Some(from) = from.filter(|w| w.is_visible().unwrap_or(false)) {
        let scale = from.scale_factor().unwrap_or(1.0);
        if let (Ok(position), Ok(size)) = (from.outer_position(), from.inner_size()) {
            let position = position.to_logical::<f64>(scale);
            let size = size.to_logical::<f64>(scale);
            config.x = Some(position.x + CASCADE);
            config.y = Some(position.y + CASCADE);
            config.width = size.width;
            config.height = size.height;
        }
    }
    /* Hidden in the menu bar, the other windows come back with the new one. */
    show(app);
    let window = WebviewWindowBuilder::from_config(app, &config)?.build()?;
    prepare(&window);
    let _ = window.set_focus();
    Ok(window)
}

/// Closing a window's last tab. Another window left, or Dispatch quitting
/// when its last window closes: this one goes, with its shells. Otherwise it
/// is the last one and Dispatch keeps running in the background, so it only
/// hides, and the page opens a fresh tab for next time (returns true).
#[tauri::command]
pub async fn close_window(window: WebviewWindow) -> bool {
    let app = window.app_handle().clone();
    if is_last(&window) && crate::keep_running(&app) {
        hide(&app);
        return true;
    }
    let _ = window.destroy();
    false
}

#[cfg(target_os = "macos")]
fn follow_active_space(window: &WebviewWindow) {
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
fn follow_active_space(_window: &WebviewWindow) {}
