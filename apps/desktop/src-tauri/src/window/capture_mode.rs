use tauri::AppHandle;

#[cfg(windows)]
pub(super) fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use super::{WindowState, capture_mode_active, main_window, overlay_window};
    use std::sync::atomic::Ordering;
    use tauri::Manager;

    if capture_mode_active(app) == enabled {
        return Ok(());
    }
    let main = main_window(app)?;
    let overlay = overlay_window(app).ok_or("capture mode requires the controls window")?;
    let state = app.state::<WindowState>();
    if state.closing.load(Ordering::SeqCst) {
        return Err("the player is closing".into());
    }
    if enabled {
        if !crate::native_shortcuts::capture_recovery_available() {
            return Err("native Escape recovery is unavailable; controls were not hidden".into());
        }
        // Set before changing focus: main's Focused event must not reactivate the overlay.
        state.capture_mode.store(true, Ordering::SeqCst);
        if let Err(error) = overlay.hide().and_then(|()| main.set_focus()) {
            state.capture_mode.store(false, Ordering::SeqCst);
            let _ = overlay.show();
            let _ = overlay.set_focus();
            return Err(error.to_string());
        }
    } else {
        // Keep recovery enabled if showing the controls fails.
        overlay.show().map_err(|error| error.to_string())?;
        state.capture_mode.store(false, Ordering::SeqCst);
        overlay.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn set_enabled(_app: &AppHandle, _enabled: bool) -> Result<(), String> {
    Err("external capture mode is currently supported on Windows only".into())
}
