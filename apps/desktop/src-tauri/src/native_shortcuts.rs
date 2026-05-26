use std::collections::HashMap;

#[cfg(windows)]
mod windows;

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
