#[cfg(feature = "mpv-embed")]
use crate::mpv_embed::{MpvEmbedSnapshot, MpvEmbedState, MpvLoadOptions};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};

mod chrome;
mod file_manager;
#[cfg(feature = "mpv-embed")]
mod mpv_overlay;
mod overlay;
mod overlay_platform;
mod resize;

pub(super) fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())
}

pub(super) fn overlay_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("overlay")
}

#[derive(Clone)]
struct WindowPlacement {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

#[derive(Default)]
pub(crate) struct WindowState {
    fullscreen_restore: Mutex<Option<WindowPlacement>>,
    always_on_top: Mutex<bool>,
}

pub(super) const MIN_MAIN_WINDOW_WIDTH: i32 = 960;
pub(super) const MIN_MAIN_WINDOW_HEIGHT: i32 = 540;

#[tauri::command]
pub(crate) fn window_minimize(app: AppHandle) -> Result<(), String> {
    chrome::minimize(app)
}

#[tauri::command]
pub(crate) fn window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    chrome::toggle_maximize(app)
}

#[tauri::command]
pub(crate) fn window_toggle_fullscreen(
    app: AppHandle,
    window_state: State<'_, WindowState>,
) -> Result<(), String> {
    chrome::toggle_fullscreen(app, window_state.inner())
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
pub(crate) fn window_close(app: AppHandle) -> Result<(), String> {
    chrome::close(app)
}

#[tauri::command]
pub(crate) fn window_focus_overlay(app: AppHandle) -> Result<(), String> {
    chrome::focus_overlay(app)
}

#[cfg(feature = "mpv-embed")]
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
