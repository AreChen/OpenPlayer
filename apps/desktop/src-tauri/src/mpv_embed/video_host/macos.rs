use std::ffi::c_void;

use super::super::*;

impl MpvVideoHost {
    pub(in crate::mpv_embed) fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        let parent_ns_view = window_appkit_ns_view(window)?;
        let Some(_mtm) = MainThreadMarker::new() else {
            return Err("macOS mpv video host must be created on the main thread".to_string());
        };

        let render_view =
            unsafe { openplayer_mpv_gl_view_create(parent_ns_view as *mut c_void) } as usize;
        if render_view == 0 {
            return Err("failed to create macOS mpv OpenGL render view".to_string());
        }

        Ok(Self { render_view })
    }

    pub(in crate::mpv_embed) fn wid(&self) -> i64 {
        self.render_view as i64
    }

    pub(in crate::mpv_embed) fn resize(&self) -> Result<(), String> {
        unsafe {
            openplayer_mpv_gl_view_resize(self.render_view_ptr());
        }
        Ok(())
    }

    pub(in crate::mpv_embed) fn render_view_ptr(&self) -> *mut c_void {
        self.render_view as *mut c_void
    }
}

impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        unsafe {
            openplayer_mpv_gl_view_remove(self.render_view_ptr());
        }
    }
}

pub(in crate::mpv_embed) fn window_appkit_ns_view(
    window: &impl HasWindowHandle,
) -> Result<usize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
        _ => Err("window operation is only wired for macOS AppKit NSView targets".to_string()),
    }
}
