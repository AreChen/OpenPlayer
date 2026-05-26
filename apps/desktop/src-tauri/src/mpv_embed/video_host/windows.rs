use super::super::*;
use super::raw_handle::window_mpv_wid;

impl MpvVideoHost {
    pub(in crate::mpv_embed) fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        let parent_hwnd = window_hwnd(window)?;
        let parent = parent_hwnd as isize as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        Self::new_with_layout(window, layout, 0)
    }

    pub(in crate::mpv_embed) fn new_with_layout(
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

    pub(in crate::mpv_embed) fn wid(&self) -> i64 {
        self.hwnd as i64
    }

    pub(in crate::mpv_embed) fn resize(&self) -> Result<(), String> {
        let parent = self.parent_hwnd as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        position_video_host(self.hwnd as HWND, layout)
    }

    pub(in crate::mpv_embed) fn resize_to_layout(
        &self,
        layout: VideoHostRect,
    ) -> Result<(), String> {
        position_video_host(self.hwnd as HWND, layout)
            .and_then(|()| apply_video_host_region(self.hwnd as HWND, layout, self.corner_radius))
    }

    pub(in crate::mpv_embed) fn set_visible(&self, visible: bool) {
        if self.hwnd == 0 {
            return;
        }
        unsafe {
            ShowWindow(self.hwnd as HWND, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    pub(in crate::mpv_embed) fn destroy(&mut self) {
        if self.hwnd == 0 {
            return;
        }
        unsafe {
            DestroyWindow(self.hwnd as HWND);
        }
        self.hwnd = 0;
        self.parent_hwnd = 0;
    }
}

pub(in crate::mpv_embed) fn position_video_host(
    hwnd: HWND,
    layout: VideoHostRect,
) -> Result<(), String> {
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

pub(in crate::mpv_embed) fn apply_video_host_region(
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

impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        self.destroy();
    }
}

pub(in crate::mpv_embed) fn window_hwnd(window: &impl HasWindowHandle) -> Result<i64, String> {
    window_mpv_wid(window)
}

pub(in crate::mpv_embed) fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(in crate::mpv_embed) fn video_host_rect(
    parent_width: i32,
    parent_height: i32,
) -> VideoHostRect {
    let width = parent_width.max(1);
    let available_height = parent_height - VIDEO_HOST_TOP_RESERVE - VIDEO_HOST_BOTTOM_RESERVE;

    VideoHostRect {
        x: 0,
        y: VIDEO_HOST_TOP_RESERVE,
        width,
        height: available_height.max(1),
    }
}
