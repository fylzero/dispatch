//! The global hotkey that shows or hides Dispatch from any app (⌘` by
//! default). There is only the one, so this keeps the accelerator that is
//! registered right now and swaps it when Settings records a new one. A new
//! combo is registered before the old one is let go, so a combo another app
//! already owns is reported and the old one keeps working.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

use crate::settings::SettingsState;

#[derive(Default)]
pub struct HotkeyState {
    /// The accelerator registered with the OS, if any.
    registered: Mutex<Option<String>>,
}

fn parse(accelerator: &str) -> Result<Shortcut, String> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|err| format!("\"{accelerator}\" is not a shortcut Dispatch understands: {err}"))
}

pub fn handle(app: &AppHandle, _shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() == ShortcutState::Pressed {
        crate::windows::toggle(app);
    }
}

fn register(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    app.global_shortcut()
        .register(parse(accelerator)?)
        .map_err(|err| {
            format!("Could not use {accelerator}: another app or the system may already own it ({err}).")
        })
}

fn unregister(app: &AppHandle, accelerator: &str) {
    if let Ok(shortcut) = parse(accelerator) {
        let _ = app.global_shortcut().unregister(shortcut);
    }
}

/// Register the saved hotkey at launch.
pub fn register_initial(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    if accelerator.is_empty() {
        return Ok(());
    }
    register(app, accelerator)?;
    *app.state::<HotkeyState>().registered.lock().unwrap() = Some(accelerator.to_string());
    Ok(())
}

/// Point the toggle at `next` (empty turns it off). On error the old one stays.
fn rebind(app: &AppHandle, next: &str) -> Result<(), String> {
    let state = app.state::<HotkeyState>();
    let mut registered = state.registered.lock().unwrap();
    let next_id = if next.is_empty() {
        None
    } else {
        Some(parse(next)?.id())
    };
    let current = registered.clone();
    let current_id = current
        .as_deref()
        .and_then(|c| parse(c).ok())
        .map(|s| s.id());
    /* "Shift+Cmd+X" and "Cmd+Shift+X" are the same keys: nothing to swap. */
    if next_id != current_id {
        if !next.is_empty() {
            register(app, next)?;
        }
        if let Some(current) = &current {
            unregister(app, current);
        }
    }
    *registered = (!next.is_empty()).then(|| next.to_string());
    Ok(())
}

/// Settings recorded a new toggle hotkey. The page saves it once this succeeds.
#[tauri::command]
pub fn set_toggle_hotkey(
    app: AppHandle,
    settings: State<SettingsState>,
    accelerator: String,
) -> Result<(), String> {
    let accelerator = accelerator.trim().to_string();
    rebind(&app, &accelerator)?;
    let snapshot = {
        let mut current = settings
            .0
            .lock()
            .map_err(|_| "Could not update settings.".to_string())?;
        current.toggle_hotkey = accelerator;
        current.clone()
    };
    crate::tray::refresh(&app, &snapshot);
    Ok(())
}

/**
 * While Settings records a new combo the current one must let go of the
 * keys: the OS hands a registered hotkey to Dispatch before the page sees
 * it, so pressing it again would hide the window instead of recording.
 */
#[tauri::command]
pub fn pause_toggle_hotkey(app: AppHandle, paused: bool) -> Result<(), String> {
    let registered = app
        .state::<HotkeyState>()
        .registered
        .lock()
        .unwrap()
        .clone();
    let Some(accelerator) = registered else {
        return Ok(());
    };
    if paused {
        unregister(&app, &accelerator);
        Ok(())
    } else {
        register(&app, &accelerator)
    }
}
