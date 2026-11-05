#[cfg(feature = "mpv-embed")]
use crate::mpv_embed::{MpvEmbedSnapshot, MpvEmbedState, MpvLoadOptions};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Manager, State, WebviewWindow};
#[cfg(not(target_os = "macos"))]
use tauri::{PhysicalPosition, PhysicalSize};

mod capture_mode;
mod chrome;
mod file_manager;
#[cfg(target_os = "macos")]
mod macos_fullscreen;
#[cfg(feature = "mpv-embed")]
mod mpv_overlay;
mod overlay;
mod overlay_platform;
mod resize;
#[cfg(feature = "window-smoke")]
pub(crate) mod smoke;

pub(super) fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())
}

pub(super) fn overlay_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("overlay")
}

#[derive(Clone)]
#[cfg(not(target_os = "macos"))]
struct WindowPlacement {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct WindowPlacement;

#[derive(Default)]
pub(crate) struct WindowState {
    fullscreen_restore: Mutex<Option<WindowPlacement>>,
    #[cfg(target_os = "macos")]
    placement_restoring: AtomicBool,
    #[cfg(target_os = "macos")]
    fullscreen_reenter: AtomicBool,
    always_on_top: Mutex<bool>,
    closing: AtomicBool,
    capture_mode: AtomicBool,
}

pub(crate) fn capture_mode_active(app: &AppHandle) -> bool {
    app.state::<WindowState>()
        .capture_mode
        .load(Ordering::SeqCst)
}

#[tauri::command]
pub(crate) async fn window_set_capture_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    let (sender, receiver) = tauri::async_runtime::channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.try_send(capture_mode::set_enabled(&handle, enabled));
    })
    .map_err(|error| error.to_string())?;
    let mut receiver = receiver;
    receiver
        .recv()
        .await
        .ok_or_else(|| "capture mode operation was interrupted".to_string())?
}

#[cfg(windows)]
pub(crate) fn restore_capture_controls(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(error) = capture_mode::set_enabled(&handle, false) {
            eprintln!("Failed to restore capture controls: {error}");
        }
    });
}

#[cfg(windows)]
pub(crate) fn request_native_close(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(error) = chrome::close(handle.clone(), &handle.state::<WindowState>()) {
            eprintln!("Failed to close native player windows: {error}");
        }
    });
}

pub(super) const MIN_MAIN_WINDOW_WIDTH: i32 = 960;
pub(super) const MIN_MAIN_WINDOW_HEIGHT: i32 = 540;

pub(super) fn begin_window_close(window_state: &WindowState) -> bool {
    window_state
        .closing
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
}

#[tauri::command]
pub(crate) fn window_minimize(app: AppHandle) -> Result<(), String> {
    chrome::minimize(app)
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub(crate) fn window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    chrome::toggle_maximize(app)
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.try_send(chrome::toggle_maximize(handle));
    })
    .map_err(|error| error.to_string())?;
    receiver
        .recv()
        .await
        .ok_or_else(|| "maximize operation was interrupted".to_string())?
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub(crate) fn window_toggle_fullscreen(
    app: AppHandle,
    window_state: State<'_, WindowState>,
) -> Result<(), String> {
    chrome::toggle_fullscreen(app, window_state.inner())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub(crate) async fn window_toggle_fullscreen(app: AppHandle) -> Result<(), String> {
    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let result = chrome::toggle_fullscreen(handle.clone(), &handle.state::<WindowState>());
        let _ = sender.try_send(result);
    })
    .map_err(|error| error.to_string())?;
    receiver
        .recv()
        .await
        .ok_or_else(|| "fullscreen operation was interrupted".to_string())?
}

#[tauri::command]
pub(crate) fn window_always_on_top_state(
    window_state: State<'_, WindowState>,
) -> Result<bool, String> {
    chrome::always_on_top_state(window_state.inner())
}

#[tauri::command]
pub(crate) fn window_toggle_always_on_top(
    app: AppHandle,
    window_state: State<'_, WindowState>,
) -> Result<bool, String> {
    chrome::toggle_always_on_top(app, window_state.inner())
}

#[tauri::command]
pub(crate) fn window_close(
    app: AppHandle,
    window_state: State<'_, WindowState>,
) -> Result<(), String> {
    chrome::close(app, window_state.inner())
}

#[tauri::command]
pub(crate) fn window_focus_overlay(app: AppHandle) -> Result<(), String> {
    chrome::focus_overlay(app)
}

#[tauri::command]
pub(crate) fn window_reveal_path(path: String) -> Result<(), String> {
    file_manager::window_reveal_path(path)
}

#[tauri::command]
pub(crate) fn window_open_directory(path: String) -> Result<(), String> {
    file_manager::window_open_directory(path)
}

#[tauri::command]
pub(crate) fn window_start_resize(app: AppHandle, direction: String) -> Result<(), String> {
    resize::window_start_resize(app, direction)
}

#[tauri::command]
pub(crate) fn window_set_resize_cursor(
    app: AppHandle,
    direction: Option<String>,
) -> Result<(), String> {
    resize::window_set_resize_cursor(app, direction)
}

#[tauri::command]
pub(crate) fn window_apply_resize_delta(
    app: AppHandle,
    direction: String,
    delta_x: f64,
    delta_y: f64,
) -> Result<(), String> {
    resize::window_apply_resize_delta(app, direction, delta_x, delta_y)
}

#[tauri::command]
pub(crate) fn window_start_drag(app: AppHandle) -> Result<(), String> {
    chrome::start_drag(app)
}

#[cfg(feature = "mpv-embed")]
#[tauri::command]
pub(crate) fn mpv_overlay_open_path(
    app: AppHandle,
    state: tauri::State<'_, MpvEmbedState>,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    mpv_overlay::open_path(
        app,
        state.inner(),
        path,
        resume_position,
        initial_volume,
        load_options,
    )
}

#[cfg(feature = "mpv-embed")]
pub(crate) fn setup_overlay_window(app: &mut tauri::App) -> Result<(), String> {
    overlay::setup_overlay_window(app)
}
