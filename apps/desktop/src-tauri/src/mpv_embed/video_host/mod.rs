#[cfg(target_os = "macos")]
mod macos;
mod raw_handle;
#[cfg(all(not(windows), not(target_os = "macos")))]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(test)]
pub(super) use raw_handle::mpv_wid_from_raw_window_handle;
#[cfg(windows)]
pub(super) use windows::window_hwnd;
#[cfg(windows)]
#[cfg(test)]
pub(super) use windows::{video_host_rect, wide_null};
