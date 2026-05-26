#[cfg(all(feature = "mpv-embed", any(windows, target_os = "macos")))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
use std::ffi::c_void;
use tauri::WebviewWindow;
#[cfg(all(feature = "mpv-embed", windows))]
use windows_sys::Win32::UI::WindowsAndMessaging::{GWLP_HWNDPARENT, SetWindowLongPtrW};

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
unsafe extern "C" {
    fn openplayer_macos_prepare_main_window(main_view: *mut c_void);
    fn openplayer_macos_prepare_overlay_window(main_view: *mut c_void, overlay_view: *mut c_void);
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
pub(super) fn prepare_macos_main_window_chrome(main: &WebviewWindow) {
    let Ok(main_view) = window_appkit_ns_view(main) else {
        return;
    };
    unsafe {
        openplayer_macos_prepare_main_window(main_view as *mut c_void);
    }
}

#[cfg(any(not(feature = "mpv-embed"), not(target_os = "macos")))]
pub(super) fn prepare_macos_main_window_chrome(_main: &WebviewWindow) {}

#[cfg(all(feature = "mpv-embed", windows))]
pub(super) fn set_overlay_owner(main: &WebviewWindow, overlay: &WebviewWindow) {
    let Ok(main_hwnd) = window_hwnd(main) else {
        return;
    };
    let Ok(overlay_hwnd) = window_hwnd(overlay) else {
        return;
    };
    unsafe {
        SetWindowLongPtrW(overlay_hwnd as _, GWLP_HWNDPARENT, main_hwnd);
    }
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
pub(super) fn set_overlay_owner(main: &WebviewWindow, overlay: &WebviewWindow) {
    let Ok(main_view) = window_appkit_ns_view(main) else {
        return;
    };
    let Ok(overlay_view) = window_appkit_ns_view(overlay) else {
        return;
    };
    unsafe {
        openplayer_macos_prepare_overlay_window(
            main_view as *mut c_void,
            overlay_view as *mut c_void,
        );
    }
}

#[cfg(all(feature = "mpv-embed", not(windows), not(target_os = "macos")))]
pub(super) fn set_overlay_owner(_main: &WebviewWindow, _overlay: &WebviewWindow) {}

#[cfg(all(feature = "mpv-embed", windows))]
fn window_hwnd(window: &impl HasWindowHandle) -> Result<isize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        _ => Err("window operation is only wired for Windows HWND targets".to_string()),
    }
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn window_appkit_ns_view(window: &impl HasWindowHandle) -> Result<usize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
        _ => Err("window operation is only wired for macOS AppKit NSView targets".to_string()),
    }
}
