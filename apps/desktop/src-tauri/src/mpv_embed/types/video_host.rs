#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VideoHostRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[cfg(windows)]
pub(crate) struct MpvVideoHost {
    pub(crate) parent_hwnd: isize,
    pub(crate) hwnd: isize,
    pub(crate) corner_radius: i32,
}

#[cfg(target_os = "macos")]
pub(crate) struct MpvVideoHost {
    pub(crate) render_view: usize,
}

#[cfg(target_os = "macos")]
pub(crate) struct MacosMpvRenderContext {
    pub(crate) ctx: usize,
    pub(crate) view: usize,
}

#[cfg(all(not(windows), not(target_os = "macos")))]
pub(crate) struct MpvVideoHost {
    pub(crate) wid: i64,
}
