use std::{cell::Cell, rc::Rc};

use gpui::{Pixels, Size, Window, px, size};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            GetClientRect, GetWindowRect, IsIconic, IsZoomed, SWP_NOACTIVATE, SWP_NOZORDER,
            SetWindowPos, WM_NCDESTROY, WM_SIZING,
        },
    },
};

use super::hwnd;

const MIN_WIDTH: f32 = 340.;
const MIN_HEIGHT: f32 = 360.;
pub(super) const COMPACT_WIDTH: Pixels = px(520.);

pub(super) fn minimum_size() -> Size<Pixels> {
    size(px(MIN_WIDTH), px(MIN_HEIGHT))
}

pub(super) fn observe_manual_resize(window: &Window) -> Rc<Cell<bool>> {
    let manual = Rc::new(Cell::new(false));
    if let Ok(handle) = hwnd(window) {
        let data = Rc::into_raw(manual.clone());
        // The subclass owns one reference until the native window is destroyed.
        if unsafe { SetWindowSubclass(handle as HWND, Some(resize_subclass), 0, data as usize) }
            == 0
        {
            unsafe { drop(Rc::from_raw(data)) };
        }
    }
    manual
}

unsafe extern "system" fn resize_subclass(
    handle: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    let manual = data as *const Cell<bool>;
    unsafe {
        match message {
            WM_SIZING => (*manual).set(true),
            WM_NCDESTROY => {
                RemoveWindowSubclass(handle, Some(resize_subclass), id);
                drop(Rc::from_raw(manual));
            }
            _ => {}
        }
        DefSubclassProc(handle, message, wparam, lparam)
    }
}

pub(super) struct FitRequest {
    handle: HWND,
    aspect: f64,
    scale: f32,
}

impl FitRequest {
    pub(super) fn new(window: &Window, aspect: f64) -> Option<Self> {
        if window.is_fullscreen() || window.is_maximized() {
            return None;
        }
        let handle = hwnd(window).ok()? as HWND;
        if unsafe { IsIconic(handle) } != 0 {
            return None;
        }
        Some(Self {
            handle,
            aspect,
            scale: window.scale_factor(),
        })
    }

    pub(super) fn apply(self) {
        fit_window(self.handle, self.aspect, self.scale);
    }
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
    if width == client.right && height == client.bottom {
        return;
    }
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
