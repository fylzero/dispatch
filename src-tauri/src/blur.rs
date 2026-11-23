use tauri::WebviewWindow;

/// Blurs whatever shows through the window's see-through background.
///
/// - macOS: a real blur radius, through the same CoreGraphics call Ghostty and
///   iTerm use (there is no public API that takes a radius).
/// - Windows: the system Acrylic blur, which has no strength, so any radius
///   above zero turns it on.
/// - Linux: no standard way to blur behind a window, so this does nothing.
#[tauri::command]
pub fn set_window_blur(window: WebviewWindow, radius: u32) -> Result<(), String> {
    apply(&window, radius.min(64))
}

#[cfg(target_os = "macos")]
fn apply(window: &WebviewWindow, radius: u32) -> Result<(), String> {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGSMainConnectionID() -> i32;
        fn CGSSetWindowBackgroundBlurRadius(connection: i32, window: u32, radius: i32) -> i32;
    }

    let target = window.clone();
    window
        .run_on_main_thread(move || {
            let Ok(ns_window) = target.ns_window() else {
                return;
            };
            if ns_window.is_null() {
                return;
            }
            // SAFETY: ns_window is the live NSWindow behind this Tauri window,
            // and AppKit is only touched here on the main thread.
            unsafe {
                let ns_window = &*(ns_window as *const AnyObject);
                let number: isize = msg_send![ns_window, windowNumber];
                CGSSetWindowBackgroundBlurRadius(
                    CGSMainConnectionID(),
                    number as u32,
                    radius as i32,
                );
            }
        })
        .map_err(|err| format!("Could not set the window blur: {err}"))
}

#[cfg(windows)]
fn apply(window: &WebviewWindow, radius: u32) -> Result<(), String> {
    use tauri::window::{Effect, EffectsBuilder};

    let effects = (radius > 0).then(|| EffectsBuilder::new().effect(Effect::Acrylic).build());
    window
        .set_effects(effects)
        .map_err(|err| format!("Could not set the window blur: {err}"))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn apply(_window: &WebviewWindow, _radius: u32) -> Result<(), String> {
    Ok(())
}
