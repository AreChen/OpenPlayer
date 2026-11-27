use gpui::{Context, Pixels, Size, Window, px, size};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::WindowsAndMessaging::{
        GetClientRect, GetWindowRect, IsIconic, IsZoomed, SWP_NOACTIVATE, SWP_NOZORDER,
        SetWindowPos,
    },
};

use super::{hwnd, player_view::PlayerView};

const MIN_WIDTH: f32 = 340.;
const MIN_HEIGHT: f32 = 360.;
pub(super) const COMPACT_WIDTH: Pixels = px(520.);

pub(super) fn minimum_size() -> Size<Pixels> {
    size(px(MIN_WIDTH), px(MIN_HEIGHT))
}

pub(super) fn fit_window_to_video(window: &Window, aspect: f64, cx: &Context<PlayerView>) {
    if window.is_fullscreen() || window.is_maximized() {
        return;
    }
    let Ok(handle) = hwnd(window) else { return };
    let scale = window.scale_factor();
    // Resize from the foreground executor after GPUI returns from event dispatch.
    cx.foreground_executor()
        .spawn(async move { fit_window(handle as HWND, aspect, scale) })
        .detach();
}

fn fit_window(handle: HWND, aspect: f64, scale: f32) {
    let mut current = RECT::default();
    let mut client = RECT::default();
    let mut monitor = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..MONITORINFO::default()
    };
    unsafe {
        if IsIconic(handle) != 0
            || IsZoomed(handle) != 0
            || GetWindowRect(handle, &mut current) == 0
            || GetClientRect(handle, &mut client) == 0
            || GetMonitorInfoW(
                MonitorFromWindow(handle, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            ) == 0
        {
            return;
        }
    }
    let frame_width = ((current.right - current.left) - client.right).max(0);
    let frame_height = ((current.bottom - current.top) - client.bottom).max(0);
    let work = monitor.rcWork;
    let available = (
        work.right - work.left - frame_width,
        work.bottom - work.top - frame_height,
    );
    if available.0 <= 0 || available.1 <= 0 {
        return;
    }
    let min = (
        (MIN_WIDTH * scale).ceil() as i32,
        (MIN_HEIGHT * scale).ceil() as i32,
    );
    let (width, height) = fit_client(aspect, client.bottom, min, available);
    let width = width + frame_width;
    let height = height + frame_height;
    let x = (current.left + (current.right - current.left - width) / 2)
        .clamp(work.left, work.right - width);
    let y = (current.top + (current.bottom - current.top - height) / 2)
        .clamp(work.top, work.bottom - height);
    unsafe {
        SetWindowPos(
            handle,
            std::ptr::null_mut(),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
}

fn fit_client(aspect: f64, current_height: i32, min: (i32, i32), max: (i32, i32)) -> (i32, i32) {
    let min_width = min.0.min(max.0);
    let min_height = min.1.min(max.1);
    let min_fitted = f64::from(min_height).max(f64::from(min_width) / aspect);
    let max_fitted = f64::from(max.1).min(f64::from(max.0) / aspect);
    // Keep the current height when possible. Extremely narrow clips may leave
    // some margins on short displays so the playback controls still fit.
    let fitted = f64::from(current_height).max(min_fitted).min(max_fitted);
    (
        ((fitted * aspect).round() as i32).clamp(min_width, max.0),
        (fitted.round() as i32).clamp(min_height, max.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_portrait_and_landscape_within_monitor_work_area() {
        let min = (340, 360);
        assert_eq!(fit_client(9. / 16., 720, min, (1920, 1000)), (405, 720));
        assert_eq!(fit_client(16. / 9., 720, min, (1920, 1000)), (1280, 720));
        assert_eq!(fit_client(16. / 9., 720, min, (1024, 620)), (1024, 576));
        assert_eq!(fit_client(1. / 3., 720, min, (1024, 620)), (340, 620));
    }
}
