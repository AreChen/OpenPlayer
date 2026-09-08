use std::collections::HashMap;

#[cfg(windows)]
mod windows;

#[cfg(all(windows, feature = "window-smoke"))]
pub(crate) use windows::recover_capture_on_escape;

#[cfg(windows)]
pub(crate) fn capture_recovery_available() -> bool {
    windows::capture_recovery_available()
}

#[cfg(windows)]
pub(crate) fn register_shell_windows(main: isize, overlay: isize) {
    windows::register_shell_windows(main, overlay);
}

#[tauri::command]
pub(crate) fn window_update_shortcuts(
    bindings: HashMap<String, Option<String>>,
) -> Result<(), String> {
    #[cfg(windows)]
    windows::update_native_shortcuts(bindings);
    #[cfg(not(windows))]
    let _ = bindings;
    Ok(())
}

#[tauri::command]
pub(crate) fn window_set_shortcuts_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    windows::set_native_shortcuts_enabled(enabled);
    #[cfg(not(windows))]
    let _ = enabled;
    Ok(())
}

#[cfg(windows)]
pub(crate) fn install_native_shortcut_hook(app: tauri::AppHandle) {
    windows::install_native_shortcut_hook(app);
}

#[cfg(not(windows))]
pub(crate) fn install_native_shortcut_hook(_app: tauri::AppHandle) {}
