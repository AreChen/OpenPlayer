use super::*;

impl MpvVideoHost {
    #[cfg(windows)]
    pub(super) fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        let parent_hwnd = window_hwnd(window)?;
        let parent = parent_hwnd as isize as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        Self::new_with_layout(window, layout, 0)
    }

    #[cfg(windows)]
    pub(super) fn new_with_layout(
        window: &impl HasWindowHandle,
        layout: VideoHostRect,
        corner_radius: i32,
    ) -> Result<Self, String> {
        let parent_hwnd = window_hwnd(window)?;
        let parent = parent_hwnd as isize as HWND;
        let class_name = wide_null("STATIC");
        let window_name = wide_null("OpenPlayer MPV Video Host");
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                layout.x,
                layout.y,
                layout.width,
                layout.height,
                parent,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };

        if hwnd.is_null() {
            return Err("failed to create native mpv child window".to_string());
        }

        unsafe {
            SetParent(hwnd, parent);
        }
        if let Err(error) = position_video_host(hwnd, layout)
            .and_then(|()| apply_video_host_region(hwnd, layout, corner_radius))
        {
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err(error);
        }
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
        }

        Ok(Self {
            parent_hwnd: parent as isize,
            hwnd: hwnd as isize,
            corner_radius,
        })
    }

    #[cfg(target_os = "macos")]
    pub(super) fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
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

    #[cfg(all(not(windows), not(target_os = "macos")))]
    pub(super) fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        Ok(Self {
            wid: window_mpv_wid(window)?,
        })
    }

    #[cfg(windows)]
    pub(super) fn wid(&self) -> i64 {
        self.hwnd as i64
    }

    #[cfg(target_os = "macos")]
    pub(super) fn wid(&self) -> i64 {
        self.render_view as i64
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    pub(super) fn wid(&self) -> i64 {
        self.wid
    }

    #[cfg(windows)]
    pub(super) fn resize(&self) -> Result<(), String> {
        let parent = self.parent_hwnd as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        position_video_host(self.hwnd as HWND, layout)
    }

    #[cfg(windows)]
    pub(super) fn resize_to_layout(&self, layout: VideoHostRect) -> Result<(), String> {
        position_video_host(self.hwnd as HWND, layout)
            .and_then(|()| apply_video_host_region(self.hwnd as HWND, layout, self.corner_radius))
    }

    #[cfg(windows)]
    pub(super) fn set_visible(&self, visible: bool) {
        if self.hwnd == 0 {
            return;
        }
        unsafe {
            ShowWindow(self.hwnd as HWND, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    #[cfg(windows)]
    pub(super) fn destroy(&mut self) {
        if self.hwnd == 0 {
            return;
        }
        unsafe {
            DestroyWindow(self.hwnd as HWND);
        }
        self.hwnd = 0;
        self.parent_hwnd = 0;
    }

    #[cfg(target_os = "macos")]
    pub(super) fn resize(&self) -> Result<(), String> {
        unsafe {
            openplayer_mpv_gl_view_resize(self.render_view_ptr());
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn render_view_ptr(&self) -> *mut c_void {
        self.render_view as *mut c_void
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    pub(super) fn resize(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(windows)]
pub(super) fn position_video_host(hwnd: HWND, layout: VideoHostRect) -> Result<(), String> {
    let result = unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOP,
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    };
    if result == 0 {
        Err("failed to position mpv child window above the video surface".to_string())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
pub(super) fn apply_video_host_region(
    hwnd: HWND,
    layout: VideoHostRect,
    corner_radius: i32,
) -> Result<(), String> {
    if corner_radius <= 0 {
        return Ok(());
    }

    let diameter = corner_radius.saturating_mul(2).max(1);
    let region = unsafe {
        CreateRoundRectRgn(
            0,
            0,
            layout.width.max(1).saturating_add(1),
            layout.height.max(1).saturating_add(1),
            diameter,
            diameter,
        )
    };
    if region.is_null() {
        return Err("failed to create rounded mpv child window region".to_string());
    }

    if unsafe { SetWindowRgn(hwnd, region, 1) } == 0 {
        unsafe {
            DeleteObject(region);
        }
        Err("failed to apply rounded mpv child window region".to_string())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(target_os = "macos")]
impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        unsafe {
            openplayer_mpv_gl_view_remove(self.render_view_ptr());
        }
    }
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) fn window_mpv_wid(window: &impl HasWindowHandle) -> Result<i64, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    mpv_wid_from_raw_window_handle(handle.as_raw())
}

#[cfg(target_os = "macos")]
pub(super) fn window_appkit_ns_view(window: &impl HasWindowHandle) -> Result<usize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
        _ => Err("window operation is only wired for macOS AppKit NSView targets".to_string()),
    }
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) fn mpv_wid_from_raw_window_handle(handle: RawWindowHandle) -> Result<i64, String> {
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
pub(super) fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    Ok(i64::from(window))
}

#[cfg(not(windows))]
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub(super) fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    if window > i64::MAX as core::ffi::c_ulong {
        Err("Xlib window id is too large for mpv wid".to_string())
    } else {
        Ok(window as i64)
    }
}

#[cfg(windows)]
pub(super) fn window_hwnd(window: &impl HasWindowHandle) -> Result<i64, String> {
    window_mpv_wid(window)
}

#[cfg(windows)]
pub(super) fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
pub(super) fn video_host_rect(parent_width: i32, parent_height: i32) -> VideoHostRect {
    let width = parent_width.max(1);
    let available_height = parent_height - VIDEO_HOST_TOP_RESERVE - VIDEO_HOST_BOTTOM_RESERVE;

    VideoHostRect {
        x: 0,
        y: VIDEO_HOST_TOP_RESERVE,
        width,
        height: available_height.max(1),
    }
}
