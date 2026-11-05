use std::{ffi::c_void, sync::atomic::Ordering};
use tauri::{AppHandle, Manager, WebviewWindow};

use super::{WindowState, chrome, overlay, overlay_window};

#[cfg(feature = "window-smoke")]
static PENDING_OBSERVERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" {
    fn openplayer_macos_capture_fullscreen_frame(window: *mut c_void) -> bool;
    fn openplayer_macos_observe_fullscreen_exit(
        window: *mut c_void,
        overlay: *mut c_void,
        callback: extern "C" fn(*mut c_void, bool),
        context: *mut c_void,
    ) -> bool;
    fn openplayer_macos_cancel_fullscreen_exit(window: *mut c_void);
    fn openplayer_macos_align_window_frames(target: *mut c_void, source: *mut c_void);
}

pub(super) fn align(target: &WebviewWindow, source: &WebviewWindow) -> Result<(), String> {
    let target = target.ns_view().map_err(|error| error.to_string())?;
    let source = source.ns_view().map_err(|error| error.to_string())?;
    unsafe { openplayer_macos_align_window_frames(target, source) };
    Ok(())
}

pub(super) fn capture(window: &WebviewWindow) -> Result<(), String> {
    let window = window.ns_window().map_err(|error| error.to_string())?;
    if unsafe { openplayer_macos_capture_fullscreen_frame(window) } {
        Ok(())
    } else {
        Err("failed to capture macOS window frame".into())
    }
}

pub(super) fn queue_toggle_during_exit(state: &WindowState) -> bool {
    if !state.fullscreen_exiting.load(Ordering::SeqCst) {
        return false;
    }
    state.fullscreen_reenter.fetch_xor(true, Ordering::SeqCst);
    true
}

pub(super) fn exit(
    app: &AppHandle,
    main: &WebviewWindow,
    state: &WindowState,
) -> Result<(), String> {
    let window = main.ns_window().map_err(|error| error.to_string())?;
    let overlay = overlay_window(app)
        .and_then(|overlay| overlay.ns_window().ok())
        .unwrap_or(std::ptr::null_mut());
    state.fullscreen_exiting.store(true, Ordering::SeqCst);
    let context = Box::into_raw(Box::new(app.clone())).cast();
    if !unsafe { openplayer_macos_observe_fullscreen_exit(window, overlay, finished, context) } {
        unsafe { drop(Box::from_raw(context.cast::<AppHandle>())) };
        state.fullscreen_exiting.store(false, Ordering::SeqCst);
        return Err("failed to observe macOS fullscreen exit".into());
    }
    #[cfg(feature = "window-smoke")]
    PENDING_OBSERVERS.fetch_add(1, Ordering::SeqCst);
    if let Err(error) = chrome::set_main_window_fullscreen(main, false) {
        unsafe { openplayer_macos_cancel_fullscreen_exit(window) };
        return Err(error);
    }
    Ok(())
}

extern "C" fn finished(context: *mut c_void, restored: bool) {
    #[cfg(feature = "window-smoke")]
    PENDING_OBSERVERS.fetch_sub(1, Ordering::SeqCst);
    let app = unsafe { Box::from_raw(context.cast::<AppHandle>()) };
    let state = app.state::<WindowState>();
    let reenter = state.fullscreen_reenter.swap(false, Ordering::SeqCst);
    state.fullscreen_exiting.store(false, Ordering::SeqCst);
    if restored && !state.closing.load(Ordering::SeqCst) {
        overlay::schedule_overlay_sync_to_main(&app);
        if reenter && let Err(error) = chrome::toggle_fullscreen((*app).clone(), state.inner()) {
            eprintln!("Failed to reenter fullscreen: {error}");
        }
    }
}

#[cfg(feature = "window-smoke")]
pub(super) fn pending_observers() -> usize {
    PENDING_OBSERVERS.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_during_exit_are_queued_by_parity() {
        let state = WindowState::default();
        assert!(!queue_toggle_during_exit(&state));
        state.fullscreen_exiting.store(true, Ordering::SeqCst);
        assert!(queue_toggle_during_exit(&state));
        assert!(state.fullscreen_reenter.load(Ordering::SeqCst));
        assert!(queue_toggle_during_exit(&state));
        assert!(!state.fullscreen_reenter.load(Ordering::SeqCst));
        assert!(queue_toggle_during_exit(&state));
        assert!(state.fullscreen_reenter.load(Ordering::SeqCst));
    }
}
