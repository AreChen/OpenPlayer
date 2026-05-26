use super::super::*;

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(in crate::mpv_embed) fn window_mpv_wid(window: &impl HasWindowHandle) -> Result<i64, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    mpv_wid_from_raw_window_handle(handle.as_raw())
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(in crate::mpv_embed) fn mpv_wid_from_raw_window_handle(
    handle: RawWindowHandle,
) -> Result<i64, String> {
    match handle {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as i64),
        RawWindowHandle::Xlib(handle) if handle.window > 0 => xlib_window_to_mpv_wid(handle.window),
        RawWindowHandle::Xcb(handle) => Ok(i64::from(handle.window.get())),
        RawWindowHandle::Wayland(_) => Err(
            "mpv embed playback currently supports Windows HWND and X11 window hosts; Wayland video host support is not implemented yet"
                .to_string(),
        ),
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as isize as i64),
        _ => Err(format!(
            "mpv embed playback currently supports Windows HWND, X11 window, and macOS AppKit NSView hosts; {} video host support is not implemented yet",
            std::env::consts::OS
        )),
    }
}

#[cfg(windows)]
fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    Ok(i64::from(window))
}

#[cfg(not(windows))]
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    if window > i64::MAX as core::ffi::c_ulong {
        Err("Xlib window id is too large for mpv wid".to_string())
    } else {
        Ok(window as i64)
    }
}
