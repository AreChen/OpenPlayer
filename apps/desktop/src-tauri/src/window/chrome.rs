#[cfg(feature = "mpv-embed")]
use crate::mpv_embed::stop_embedded_player_for_close;
use tauri::{AppHandle, Position, Size, WebviewWindow};

use super::{
    WindowPlacement, WindowState, begin_window_close, main_window,
    overlay::{focus_overlay_window, schedule_overlay_sync_to_main},
    overlay_window,
};

pub(super) fn minimize(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = overlay_window(&app) {
        let _ = overlay.minimize();
    }
    main_window(&app)?
        .minimize()
        .map_err(|error| error.to_string())
}

pub(super) fn toggle_maximize(app: AppHandle) -> Result<(), String> {
    let main = main_window(&app)?;
    if main.is_maximized().map_err(|error| error.to_string())? {
        main.unmaximize().map_err(|error| error.to_string())?
    } else {
        main.maximize().map_err(|error| error.to_string())?
    }
    schedule_overlay_sync_to_main(&app);
    Ok(())
}

pub(super) fn toggle_fullscreen(app: AppHandle, window_state: &WindowState) -> Result<(), String> {
    let main = main_window(&app)?;
    let mut fullscreen_restore = window_state
        .fullscreen_restore
        .lock()
        .map_err(|_| "window state lock failed".to_string())?;
    let has_restore_placement = fullscreen_restore.is_some();
    let is_fullscreen =
        has_restore_placement || main.is_fullscreen().map_err(|error| error.to_string())?;
    if is_fullscreen {
        if let Some(placement) = fullscreen_restore.take() {
            drop(fullscreen_restore);
            restore_window_after_fullscreen(&main, placement)?;
        } else {
            drop(fullscreen_restore);
            set_main_window_fullscreen(&main, false)?;
        }
    } else {
        let placement = capture_window_placement(&main)?;
        set_main_window_fullscreen(&main, true)?;
        *fullscreen_restore = Some(placement);
        drop(fullscreen_restore);
    }

    schedule_overlay_sync_to_main(&app);
    Ok(())
}

pub(super) fn always_on_top_state(window_state: &WindowState) -> Result<bool, String> {
    window_state
        .always_on_top
        .lock()
        .map(|state| *state)
        .map_err(|_| "window state lock failed".to_string())
}

pub(super) fn toggle_always_on_top(
    app: AppHandle,
    window_state: &WindowState,
) -> Result<bool, String> {
    let mut always_on_top = window_state
        .always_on_top
        .lock()
        .map_err(|_| "window state lock failed".to_string())?;
    let enabled = !*always_on_top;
    set_window_always_on_top(&app, enabled)?;
    *always_on_top = enabled;
    drop(always_on_top);
    focus_overlay_window(&app);
    Ok(enabled)
}

pub(super) fn close(app: AppHandle, window_state: &WindowState) -> Result<(), String> {
    if !begin_window_close(window_state) {
        return Ok(());
    }

    #[cfg(feature = "mpv-embed")]
    let _ = stop_embedded_player_for_close(&app);

    if let Some(overlay) = overlay_window(&app) {
        let _ = overlay.close();
    }
    main_window(&app)?
        .close()
        .map_err(|error| error.to_string())
}

pub(super) fn focus_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = overlay_window(&app) {
        overlay.set_focus().map_err(|error| error.to_string())
    } else {
        main_window(&app)?
            .set_focus()
            .map_err(|error| error.to_string())
    }
}

pub(super) fn start_drag(app: AppHandle) -> Result<(), String> {
    main_window(&app)?
        .start_dragging()
        .map_err(|error| error.to_string())
}

fn set_window_always_on_top(app: &AppHandle, enabled: bool) -> Result<(), String> {
    main_window(app)?
        .set_always_on_top(enabled)
        .map_err(|error| error.to_string())?;
    if let Some(overlay) = overlay_window(app) {
        overlay
            .set_always_on_top(enabled)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn capture_window_placement(window: &WebviewWindow) -> Result<WindowPlacement, String> {
    Ok(WindowPlacement {
        position: window.outer_position().map_err(|error| error.to_string())?,
        size: window.outer_size().map_err(|error| error.to_string())?,
        maximized: window.is_maximized().map_err(|error| error.to_string())?,
    })
}

#[cfg(target_os = "macos")]
fn set_main_window_fullscreen(window: &WebviewWindow, fullscreen: bool) -> Result<(), String> {
    super::overlay_platform::prepare_macos_main_window_chrome(window);
    window
        .set_fullscreen(fullscreen)
        .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "macos"))]
fn set_main_window_fullscreen(window: &WebviewWindow, fullscreen: bool) -> Result<(), String> {
    window
        .set_fullscreen(fullscreen)
        .map_err(|error| error.to_string())
}

fn restore_window_after_fullscreen(
    window: &WebviewWindow,
    placement: WindowPlacement,
) -> Result<(), String> {
    set_main_window_fullscreen(window, false)?;

    if placement.maximized {
        window.maximize().map_err(|error| error.to_string())?;
    } else {
        window
            .set_position(Position::Physical(placement.position))
            .map_err(|error| error.to_string())?;
        window
            .set_size(Size::Physical(placement.size))
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}
