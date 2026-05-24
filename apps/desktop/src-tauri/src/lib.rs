#[cfg(all(feature = "mpv-embed", any(windows, target_os = "macos")))]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
use std::ffi::c_void;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    thread,
    time::Duration,
};
#[cfg(windows)]
use std::{
    collections::HashSet,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
#[cfg(windows)]
use tauri::Emitter;
#[cfg(feature = "mpv-embed")]
use tauri::utils::config::Color;
use tauri::{
    AppHandle, CursorIcon, Manager, PhysicalPosition, PhysicalSize, Position, Size, State,
    WebviewWindow,
};
#[cfg(feature = "mpv-embed")]
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_runtime::ResizeDirection;
#[cfg(all(feature = "mpv-embed", windows))]
use windows_sys::Win32::UI::WindowsAndMessaging::{GWLP_HWNDPARENT, SetWindowLongPtrW};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_LWIN,
            VK_MENU, VK_OEM_5, VK_OEM_COMMA, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SPACE,
            VK_UP,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
            KBDLLHOOKSTRUCT, MSG, SetWindowsHookExW, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
            WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

#[cfg(feature = "mpv-embed")]
use tauri::WindowEvent;

#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;

#[cfg(feature = "mpv-embed")]
mod mpv_embed;

mod appearance_store;
mod media_paths;
mod platform_support;
mod playback_store;
mod shell_preview;

use appearance_store::{
    AppearanceStoreState, appearance_import_theme_plugin, appearance_reset,
    appearance_set_accent_override, appearance_set_plugin_enabled, appearance_set_theme,
    appearance_state, preferences_set_incognito_mode, preferences_set_language_mode,
    preferences_set_quiet_keyboard_controls, preferences_state,
};
use media_paths::{
    StartupMediaState, media_files_from_paths, media_files_in_directory, startup_media_paths,
};
#[cfg(feature = "mpv-embed")]
use mpv_embed::{
    MpvEmbedSnapshot, MpvEmbedState, mpv_embed_add_subtitle, mpv_embed_frame_back_step,
    mpv_embed_frame_step, mpv_embed_pause, mpv_embed_play, mpv_embed_seek, mpv_embed_select_track,
    mpv_embed_set_hwdec, mpv_embed_set_loop_file, mpv_embed_set_speed,
    mpv_embed_set_subtitle_delay, mpv_embed_set_video_fill, mpv_embed_set_volume,
    mpv_embed_snapshot, mpv_embed_stop,
};
use platform_support::{platform_support, prepare_platform_runtime};
use playback_store::{
    PlaybackStoreState, history_clear, history_list, history_remember, history_resume_position,
    playback_media_settings, playback_media_settings_update, playback_settings_state,
    playback_settings_update,
};
use shell_preview::{
    shell_preview_formats, shell_preview_open_default_apps_settings, shell_preview_register_formats,
};

#[cfg(feature = "mpv-smoke")]
pub use mpv_smoke::{MpvSmokeReport, create_headless_probe};

fn main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())
}

fn overlay_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("overlay")
}

#[derive(Clone)]
struct WindowPlacement {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

#[derive(Default)]
struct WindowState {
    fullscreen_restore: Mutex<Option<WindowPlacement>>,
    always_on_top: Mutex<bool>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AppVersionInfo {
    name: &'static str,
    version: &'static str,
    license: &'static str,
    repository: &'static str,
    releases_url: &'static str,
}

const OPENPLAYER_REPOSITORY_URL: &str = "https://github.com/AreChen/OpenPlayer";
const OPENPLAYER_RELEASES_URL: &str = "https://github.com/AreChen/OpenPlayer/releases/latest";

#[cfg(windows)]
const NATIVE_SHORTCUT_EVENT: &str = "openplayer-native-shortcut";

#[cfg(windows)]
struct NativeShortcutState {
    app: AppHandle,
    shortcuts: Mutex<HashMap<String, String>>,
    enabled: AtomicBool,
    pressed_keys: Mutex<HashSet<u32>>,
    hook: Mutex<Option<usize>>,
}

#[cfg(windows)]
static NATIVE_SHORTCUT_STATE: OnceLock<NativeShortcutState> = OnceLock::new();
#[cfg(feature = "mpv-embed")]
static MPV_VIDEO_HOST_SYNC_PENDING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

const MIN_MAIN_WINDOW_WIDTH: i32 = 960;
const MIN_MAIN_WINDOW_HEIGHT: i32 = 540;

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
unsafe extern "C" {
    fn openplayer_macos_prepare_main_window(main_view: *mut c_void);
    fn openplayer_macos_prepare_overlay_window(main_view: *mut c_void, overlay_view: *mut c_void);
}

#[tauri::command]
fn window_update_shortcuts(bindings: HashMap<String, Option<String>>) -> Result<(), String> {
    #[cfg(windows)]
    update_native_shortcuts(bindings);
    #[cfg(not(windows))]
    let _ = bindings;
    Ok(())
}

#[tauri::command]
fn window_set_shortcuts_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    set_native_shortcuts_enabled(enabled);
    #[cfg(not(windows))]
    let _ = enabled;
    Ok(())
}

#[tauri::command]
fn app_version() -> AppVersionInfo {
    AppVersionInfo {
        name: "OpenPlayer",
        version: env!("CARGO_PKG_VERSION"),
        license: env!("CARGO_PKG_LICENSE"),
        repository: OPENPLAYER_REPOSITORY_URL,
        releases_url: OPENPLAYER_RELEASES_URL,
    }
}

#[tauri::command]
fn app_open_url(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !is_safe_external_url(trimmed) {
        return Err("invalid external url".to_string());
    }

    open_external_url(trimmed)
}

fn is_safe_external_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !url.chars().any(char::is_whitespace)
}

#[cfg(windows)]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("rundll32")
        .arg("url.dll,FileProtocolHandler")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open url: {error}"))
}

#[cfg(target_os = "macos")]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open url: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_external_url(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open url: {error}"))
}

#[cfg(windows)]
fn install_native_shortcut_hook(app: AppHandle) {
    let _ = NATIVE_SHORTCUT_STATE.set(NativeShortcutState {
        app,
        shortcuts: Mutex::new(HashMap::new()),
        enabled: AtomicBool::new(true),
        pressed_keys: Mutex::new(HashSet::new()),
        hook: Mutex::new(None),
    });

    thread::spawn(|| unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(native_shortcut_keyboard_proc),
            module,
            0,
        );
        if hook.is_null() {
            return;
        }

        if let Some(state) = NATIVE_SHORTCUT_STATE.get()
            && let Ok(mut stored_hook) = state.hook.lock()
        {
            *stored_hook = Some(hook as usize);
        }

        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {}
    });
}

#[cfg(not(windows))]
fn install_native_shortcut_hook(_app: AppHandle) {}

#[cfg(windows)]
fn update_native_shortcuts(bindings: HashMap<String, Option<String>>) {
    let Some(state) = NATIVE_SHORTCUT_STATE.get() else {
        return;
    };
    let Ok(mut shortcuts) = state.shortcuts.lock() else {
        return;
    };

    shortcuts.clear();
    for (action, chord) in bindings {
        if let Some(chord) = chord {
            shortcuts.insert(chord, action);
        }
    }
}

#[cfg(windows)]
fn set_native_shortcuts_enabled(enabled: bool) {
    if let Some(state) = NATIVE_SHORTCUT_STATE.get() {
        state.enabled.store(enabled, Ordering::SeqCst);
    }
}

#[cfg(windows)]
unsafe extern "system" fn native_shortcut_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    }

    let message = wparam as u32;
    let is_key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    let is_key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
    if !is_key_down && !is_key_up {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    }

    let Some(state) = NATIVE_SHORTCUT_STATE.get() else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    };
    let key = unsafe { *(lparam as *const KBDLLHOOKSTRUCT) };
    if is_key_up && let Ok(mut pressed_keys) = state.pressed_keys.lock() {
        pressed_keys.remove(&key.vkCode);
    }

    if !state.enabled.load(Ordering::SeqCst) || !is_openplayer_foreground() {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    }

    let Some(chord) = native_shortcut_chord(key.vkCode) else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    };
    let action = state
        .shortcuts
        .lock()
        .ok()
        .and_then(|shortcuts| shortcuts.get(&chord).cloned());
    let Some(action) = action else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    };

    if is_key_up {
        return 1;
    }

    let first_press = state
        .pressed_keys
        .lock()
        .map(|mut pressed_keys| pressed_keys.insert(key.vkCode))
        .unwrap_or(true);
    if first_press {
        let _ = state.app.emit_to("overlay", NATIVE_SHORTCUT_EVENT, action);
    }

    1
}

#[cfg(windows)]
fn is_openplayer_foreground() -> bool {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_null() {
        return false;
    }

    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(foreground, &mut process_id);
    }
    process_id == std::process::id()
}

#[cfg(windows)]
fn native_shortcut_chord(vk_code: u32) -> Option<String> {
    let key = native_shortcut_key(vk_code)?;
    let mut parts = Vec::new();

    if native_modifier_down(VK_CONTROL) {
        parts.push("Ctrl");
    }
    if native_modifier_down(VK_LWIN) || native_modifier_down(VK_RWIN) {
        parts.push("Meta");
    }
    if native_modifier_down(VK_MENU) {
        parts.push("Alt");
    }
    if native_modifier_down(VK_SHIFT) {
        parts.push("Shift");
    }

    parts.push(key);
    Some(parts.join("+"))
}

#[cfg(windows)]
fn native_modifier_down(vkey: u16) -> bool {
    unsafe { GetAsyncKeyState(vkey as i32) < 0 }
}

#[cfg(windows)]
fn native_shortcut_key(vk_code: u32) -> Option<&'static str> {
    match vk_code {
        0x30 => Some("0"),
        0x31 => Some("1"),
        0x32 => Some("2"),
        0x33 => Some("3"),
        0x34 => Some("4"),
        0x35 => Some("5"),
        0x36 => Some("6"),
        0x37 => Some("7"),
        0x38 => Some("8"),
        0x39 => Some("9"),
        0x41 => Some("A"),
        0x42 => Some("B"),
        0x43 => Some("C"),
        0x44 => Some("D"),
        0x45 => Some("E"),
        0x46 => Some("F"),
        0x47 => Some("G"),
        0x48 => Some("H"),
        0x49 => Some("I"),
        0x4A => Some("J"),
        0x4B => Some("K"),
        0x4C => Some("L"),
        0x4D => Some("M"),
        0x4E => Some("N"),
        0x4F => Some("O"),
        0x50 => Some("P"),
        0x51 => Some("Q"),
        0x52 => Some("R"),
        0x53 => Some("S"),
        0x54 => Some("T"),
        0x55 => Some("U"),
        0x56 => Some("V"),
        0x57 => Some("W"),
        0x58 => Some("X"),
        0x59 => Some("Y"),
        0x5A => Some("Z"),
        value if value == VK_BACK as u32 => Some("Backspace"),
        value if value == VK_DELETE as u32 => Some("Delete"),
        value if value == VK_DOWN as u32 => Some("ArrowDown"),
        value if value == VK_ESCAPE as u32 => Some("Escape"),
        value if value == VK_LEFT as u32 => Some("ArrowLeft"),
        value if value == VK_OEM_5 as u32 => Some("\\"),
        value if value == VK_OEM_COMMA as u32 => Some(","),
        value if value == VK_RETURN as u32 => Some("Enter"),
        value if value == VK_RIGHT as u32 => Some("ArrowRight"),
        value if value == VK_SPACE as u32 => Some("Space"),
        value if value == VK_UP as u32 => Some("ArrowUp"),
        _ => None,
    }
}

#[tauri::command]
fn window_minimize(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = overlay_window(&app) {
        let _ = overlay.minimize();
    }
    main_window(&app)?
        .minimize()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn window_toggle_maximize(app: AppHandle) -> Result<(), String> {
    let main = main_window(&app)?;
    if main.is_maximized().map_err(|error| error.to_string())? {
        main.unmaximize().map_err(|error| error.to_string())?
    } else {
        main.maximize().map_err(|error| error.to_string())?
    }
    schedule_overlay_sync_to_main(&app);
    Ok(())
}

#[tauri::command]
fn window_toggle_fullscreen(
    app: AppHandle,
    window_state: State<'_, WindowState>,
) -> Result<(), String> {
    let main = main_window(&app)?;
    let mut fullscreen_restore = window_state
        .fullscreen_restore
        .lock()
        .map_err(|_| "window state lock failed".to_string())?;
    let has_restore_placement = fullscreen_restore.is_some();
    let is_fullscreen =
        has_restore_placement || main.is_fullscreen().map_err(|error| error.to_string())?;
    if is_fullscreen {
        if let Some(placement) = fullscreen_restore.take() {
            drop(fullscreen_restore);
            restore_window_after_fullscreen(&main, placement)?;
        } else {
            drop(fullscreen_restore);
            set_main_window_fullscreen(&main, false)?;
        }
    } else {
        let placement = capture_window_placement(&main)?;
        set_main_window_fullscreen(&main, true)?;
        *fullscreen_restore = Some(placement);
        drop(fullscreen_restore);
    }

    schedule_overlay_sync_to_main(&app);
    Ok(())
}

#[tauri::command]
fn window_always_on_top_state(window_state: State<'_, WindowState>) -> Result<bool, String> {
    window_state
        .always_on_top
        .lock()
        .map(|state| *state)
        .map_err(|_| "window state lock failed".to_string())
}

#[tauri::command]
fn window_toggle_always_on_top(
    app: AppHandle,
    window_state: State<'_, WindowState>,
) -> Result<bool, String> {
    let mut always_on_top = window_state
        .always_on_top
        .lock()
        .map_err(|_| "window state lock failed".to_string())?;
    let enabled = !*always_on_top;
    set_window_always_on_top(&app, enabled)?;
    *always_on_top = enabled;
    drop(always_on_top);
    focus_overlay_window(&app);
    Ok(enabled)
}

#[tauri::command]
fn window_close(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = overlay_window(&app) {
        let _ = overlay.close();
    }
    main_window(&app)?
        .close()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn window_reveal_path(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("file path is empty".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.exists() {
        return Err(format!("file path does not exist: {}", path.display()));
    }

    reveal_path_in_file_manager(&path)
}

fn set_window_always_on_top(app: &AppHandle, enabled: bool) -> Result<(), String> {
    main_window(app)?
        .set_always_on_top(enabled)
        .map_err(|error| error.to_string())?;
    if let Some(overlay) = overlay_window(app) {
        overlay
            .set_always_on_top(enabled)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open file location: {error}"))
}

#[cfg(target_os = "macos")]
fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open file location: {error}"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_path_in_file_manager(path: &Path) -> Result<(), String> {
    let target = path
        .parent()
        .filter(|parent| parent.exists())
        .unwrap_or(path);
    std::process::Command::new("xdg-open")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open file location: {error}"))
}

fn sync_overlay_to_main(app: &AppHandle) {
    sync_overlay_to_main_with_focus(app, true);
}

fn sync_overlay_to_main_without_focus(app: &AppHandle) {
    sync_overlay_to_main_with_focus(app, false);
}

fn sync_overlay_to_main_with_focus(app: &AppHandle, focus_overlay: bool) {
    let Ok(main) = main_window(app) else {
        return;
    };
    let Some(overlay) = overlay_window(app) else {
        return;
    };
    let Ok(position) = main.outer_position() else {
        return;
    };
    let Ok(size) = main.outer_size() else {
        return;
    };
    let _ = overlay.set_position(Position::Physical(PhysicalPosition {
        x: position.x,
        y: position.y,
    }));
    let _ = overlay.set_size(Size::Physical(PhysicalSize {
        width: size.width,
        height: size.height,
    }));
    if focus_overlay {
        focus_overlay_window(app);
    }
}

fn focus_overlay_window(app: &AppHandle) {
    if let Some(overlay) = overlay_window(app) {
        let _ = overlay.set_focus();
    }
}

#[tauri::command]
fn window_focus_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = overlay_window(&app) {
        overlay.set_focus().map_err(|error| error.to_string())
    } else {
        main_window(&app)?
            .set_focus()
            .map_err(|error| error.to_string())
    }
}

fn schedule_overlay_sync_to_main(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        for delay in [
            Duration::from_millis(40),
            Duration::from_millis(120),
            Duration::from_millis(260),
        ] {
            thread::sleep(delay);
            let app_for_sync = app.clone();
            let _ = app.run_on_main_thread(move || sync_overlay_to_main(&app_for_sync));
        }
    });
}

#[cfg(feature = "mpv-embed")]
fn sync_mpv_video_host(app: &AppHandle) {
    let state = app.state::<MpvEmbedState>();
    let _ = state.resize_video_host();
}

#[cfg(feature = "mpv-embed")]
fn schedule_mpv_video_host_sync(app: &AppHandle) {
    if MPV_VIDEO_HOST_SYNC_PENDING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }

    let app = app.clone();
    thread::spawn(move || {
        for delay in [
            Duration::from_millis(16),
            Duration::from_millis(80),
            Duration::from_millis(180),
        ] {
            thread::sleep(delay);
            let app_for_sync = app.clone();
            let _ = app.run_on_main_thread(move || sync_mpv_video_host(&app_for_sync));
        }
        MPV_VIDEO_HOST_SYNC_PENDING.store(false, std::sync::atomic::Ordering::SeqCst);
    });
}

fn capture_window_placement(window: &WebviewWindow) -> Result<WindowPlacement, String> {
    Ok(WindowPlacement {
        position: window.outer_position().map_err(|error| error.to_string())?,
        size: window.outer_size().map_err(|error| error.to_string())?,
        maximized: window.is_maximized().map_err(|error| error.to_string())?,
    })
}

#[cfg(target_os = "macos")]
fn set_main_window_fullscreen(window: &WebviewWindow, fullscreen: bool) -> Result<(), String> {
    prepare_macos_main_window_chrome(window);
    window
        .set_fullscreen(fullscreen)
        .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "macos"))]
fn set_main_window_fullscreen(window: &WebviewWindow, fullscreen: bool) -> Result<(), String> {
    window
        .set_fullscreen(fullscreen)
        .map_err(|error| error.to_string())
}

fn restore_window_after_fullscreen(
    window: &WebviewWindow,
    placement: WindowPlacement,
) -> Result<(), String> {
    set_main_window_fullscreen(window, false)?;

    if placement.maximized {
        window.maximize().map_err(|error| error.to_string())?;
    } else {
        window
            .set_position(Position::Physical(placement.position))
            .map_err(|error| error.to_string())?;
        window
            .set_size(Size::Physical(placement.size))
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[cfg(all(feature = "mpv-embed", windows))]
fn set_overlay_owner(main: &WebviewWindow, overlay: &WebviewWindow) {
    let Ok(main_hwnd) = window_hwnd(main) else {
        return;
    };
    let Ok(overlay_hwnd) = window_hwnd(overlay) else {
        return;
    };
    unsafe {
        SetWindowLongPtrW(overlay_hwnd as _, GWLP_HWNDPARENT, main_hwnd);
    }
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn prepare_macos_main_window_chrome(main: &WebviewWindow) {
    let Ok(main_view) = window_appkit_ns_view(main) else {
        return;
    };
    unsafe {
        openplayer_macos_prepare_main_window(main_view as *mut c_void);
    }
}

#[cfg(any(not(feature = "mpv-embed"), not(target_os = "macos")))]
fn prepare_macos_main_window_chrome(_main: &WebviewWindow) {}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn set_overlay_owner(main: &WebviewWindow, overlay: &WebviewWindow) {
    let Ok(main_view) = window_appkit_ns_view(main) else {
        return;
    };
    let Ok(overlay_view) = window_appkit_ns_view(overlay) else {
        return;
    };
    unsafe {
        openplayer_macos_prepare_overlay_window(
            main_view as *mut c_void,
            overlay_view as *mut c_void,
        );
    }
}

#[cfg(all(feature = "mpv-embed", not(windows)))]
#[cfg(not(target_os = "macos"))]
fn set_overlay_owner(_main: &WebviewWindow, _overlay: &WebviewWindow) {}

#[cfg(all(feature = "mpv-embed", windows))]
fn window_hwnd(window: &impl HasWindowHandle) -> Result<isize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        _ => Err("window operation is only wired for Windows HWND targets".to_string()),
    }
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn window_appkit_ns_view(window: &impl HasWindowHandle) -> Result<usize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
        _ => Err("window operation is only wired for macOS AppKit NSView targets".to_string()),
    }
}

#[cfg(feature = "mpv-embed")]
#[tauri::command]
fn window_start_drag(app: AppHandle) -> Result<(), String> {
    main_window(&app)?
        .start_dragging()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn window_start_resize(app: AppHandle, direction: String) -> Result<(), String> {
    let direction = resize_direction_from_str(&direction)?;

    main_window(&app)?
        .as_ref()
        .window()
        .start_resize_dragging(direction)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn window_set_resize_cursor(app: AppHandle, direction: Option<String>) -> Result<(), String> {
    let icon = match direction.as_deref() {
        Some("East") => CursorIcon::EResize,
        Some("North") => CursorIcon::NResize,
        Some("NorthEast") => CursorIcon::NeResize,
        Some("NorthWest") => CursorIcon::NwResize,
        Some("South") => CursorIcon::SResize,
        Some("SouthEast") => CursorIcon::SeResize,
        Some("SouthWest") => CursorIcon::SwResize,
        Some("West") => CursorIcon::WResize,
        Some("Default") | None => CursorIcon::Default,
        Some(direction) => return Err(format!("invalid resize cursor direction: {direction}")),
    };

    main_window(&app)?
        .set_cursor_icon(icon)
        .map_err(|error| error.to_string())?;
    if let Some(overlay) = overlay_window(&app) {
        overlay
            .set_cursor_icon(icon)
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn resize_direction_from_str(direction: &str) -> Result<ResizeDirection, String> {
    Ok(match direction {
        "East" => ResizeDirection::East,
        "North" => ResizeDirection::North,
        "NorthEast" => ResizeDirection::NorthEast,
        "NorthWest" => ResizeDirection::NorthWest,
        "South" => ResizeDirection::South,
        "SouthEast" => ResizeDirection::SouthEast,
        "SouthWest" => ResizeDirection::SouthWest,
        "West" => ResizeDirection::West,
        _ => return Err(format!("invalid resize direction: {direction}")),
    })
}

#[tauri::command]
fn window_apply_resize_delta(
    app: AppHandle,
    direction: String,
    delta_x: f64,
    delta_y: f64,
) -> Result<(), String> {
    if !delta_x.is_finite() || !delta_y.is_finite() {
        return Err("invalid resize delta".to_string());
    }

    let direction = resize_direction_from_str(&direction)?;
    let main = main_window(&app)?;
    if main.is_fullscreen().map_err(|error| error.to_string())?
        || main.is_maximized().map_err(|error| error.to_string())?
    {
        return Ok(());
    }

    let position = main.outer_position().map_err(|error| error.to_string())?;
    let size = main.outer_size().map_err(|error| error.to_string())?;
    let old_width = size.width as i32;
    let old_height = size.height as i32;
    let dx = delta_x.round() as i32;
    let dy = delta_y.round() as i32;
    let mut x = position.x;
    let mut y = position.y;
    let mut width = old_width;
    let mut height = old_height;

    if resize_direction_has_west_edge(direction) {
        x += dx;
        width -= dx;
    }
    if resize_direction_has_east_edge(direction) {
        width += dx;
    }
    if resize_direction_has_north_edge(direction) {
        y += dy;
        height -= dy;
    }
    if resize_direction_has_south_edge(direction) {
        height += dy;
    }

    if width < MIN_MAIN_WINDOW_WIDTH {
        if resize_direction_has_west_edge(direction) {
            x -= MIN_MAIN_WINDOW_WIDTH - width;
        }
        width = MIN_MAIN_WINDOW_WIDTH;
    }
    if height < MIN_MAIN_WINDOW_HEIGHT {
        if resize_direction_has_north_edge(direction) {
            y -= MIN_MAIN_WINDOW_HEIGHT - height;
        }
        height = MIN_MAIN_WINDOW_HEIGHT;
    }

    main.set_position(Position::Physical(PhysicalPosition { x, y }))
        .map_err(|error| error.to_string())?;
    main.set_size(Size::Physical(PhysicalSize {
        width: width as u32,
        height: height as u32,
    }))
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn resize_direction_has_west_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::West | ResizeDirection::NorthWest | ResizeDirection::SouthWest
    )
}

fn resize_direction_has_east_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::East | ResizeDirection::NorthEast | ResizeDirection::SouthEast
    )
}

fn resize_direction_has_north_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::North | ResizeDirection::NorthEast | ResizeDirection::NorthWest
    )
}

fn resize_direction_has_south_edge(direction: ResizeDirection) -> bool {
    matches!(
        direction,
        ResizeDirection::South | ResizeDirection::SouthEast | ResizeDirection::SouthWest
    )
}

#[cfg(feature = "mpv-embed")]
#[tauri::command]
fn mpv_overlay_open_path(
    app: AppHandle,
    state: tauri::State<'_, MpvEmbedState>,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
) -> Result<MpvEmbedSnapshot, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = state;
        return open_path_for_main_window_on_main_thread(
            app,
            path,
            resume_position,
            initial_volume,
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        let main = main_window(&app)?;
        sync_overlay_to_main(&app);
        mpv_embed::open_path_for_window(&main, state.inner(), path, resume_position, initial_volume)
    }
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn open_path_for_main_window_on_main_thread(
    app: AppHandle,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
) -> Result<MpvEmbedSnapshot, String> {
    if objc2::MainThreadMarker::new().is_some() {
        return open_path_for_main_window_now(&app, path, resume_position, initial_volume);
    }

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let app_for_open = app.clone();
    app.run_on_main_thread(move || {
        let result =
            open_path_for_main_window_now(&app_for_open, path, resume_position, initial_volume);
        let _ = sender.send(result);
    })
    .map_err(|error| format!("failed to schedule macOS mpv AppKit host setup: {error}"))?;

    receiver
        .recv()
        .map_err(|_| "macOS mpv AppKit host setup did not return a result".to_string())?
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn open_path_for_main_window_now(
    app: &AppHandle,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
) -> Result<MpvEmbedSnapshot, String> {
    let main = main_window(app)?;
    sync_overlay_to_main(app);
    let state = app.state::<MpvEmbedState>();
    mpv_embed::open_path_for_window(&main, state.inner(), path, resume_position, initial_volume)
}

#[cfg(not(feature = "mpv-embed"))]
pub fn run() {
    prepare_platform_runtime();
    tauri::Builder::default()
        .manage(WindowState::default())
        .manage(StartupMediaState::from_args(std::env::args_os()))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(AppearanceStoreState::open(app.handle()));
            app.manage(PlaybackStoreState::open(app.handle()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_update_shortcuts,
            window_set_shortcuts_enabled,
            app_version,
            app_open_url,
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_always_on_top_state,
            window_toggle_always_on_top,
            window_focus_overlay,
            window_start_resize,
            window_set_resize_cursor,
            window_apply_resize_delta,
            window_close,
            window_reveal_path,
            media_files_from_paths,
            media_files_in_directory,
            startup_media_paths,
            platform_support,
            appearance_state,
            appearance_set_theme,
            appearance_set_accent_override,
            appearance_import_theme_plugin,
            appearance_set_plugin_enabled,
            appearance_reset,
            preferences_state,
            preferences_set_incognito_mode,
            preferences_set_quiet_keyboard_controls,
            preferences_set_language_mode,
            shell_preview_formats,
            shell_preview_open_default_apps_settings,
            shell_preview_register_formats,
            history_list,
            history_remember,
            history_resume_position,
            history_clear,
            playback_settings_state,
            playback_settings_update,
            playback_media_settings,
            playback_media_settings_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}

#[cfg(feature = "mpv-embed")]
pub fn run() {
    prepare_platform_runtime();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(WindowState::default())
        .manage(MpvEmbedState::default())
        .manage(StartupMediaState::from_args(std::env::args_os()))
        .setup(|app| {
            app.manage(AppearanceStoreState::open(app.handle()));
            app.manage(PlaybackStoreState::open(app.handle()));
            install_native_shortcut_hook(app.handle().clone());
            if let Some(window) = app.get_webview_window("main") {
                prepare_macos_main_window_chrome(&window);
                let overlay = WebviewWindowBuilder::new(
                    app,
                    "overlay",
                    WebviewUrl::App("index.html?surface=overlay".into()),
                )
                .title("OpenPlayer Controls")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .skip_taskbar(true)
                .background_color(Color(0, 0, 0, 0))
                .visible(false)
                .build()
                .map_err(|error| format!("failed to create overlay controls window: {error}"))?;
                let _ = overlay.set_background_color(Some(Color(0, 0, 0, 0)));
                set_overlay_owner(&window, &overlay);

                let app_handle = app.handle().clone();
                sync_overlay_to_main(&app_handle);
                let _ = overlay.show();
                set_overlay_owner(&window, &overlay);
                window.on_window_event(move |event| {
                    if matches!(
                        event,
                        WindowEvent::Moved(_)
                            | WindowEvent::Resized(_)
                            | WindowEvent::ScaleFactorChanged { .. }
                    ) {
                        sync_overlay_to_main_without_focus(&app_handle);
                        sync_mpv_video_host(&app_handle);
                        schedule_mpv_video_host_sync(&app_handle);
                    }
                    if matches!(event, WindowEvent::Focused(true)) {
                        focus_overlay_window(&app_handle);
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_update_shortcuts,
            window_set_shortcuts_enabled,
            app_version,
            app_open_url,
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_always_on_top_state,
            window_toggle_always_on_top,
            window_close,
            window_focus_overlay,
            window_start_drag,
            window_start_resize,
            window_set_resize_cursor,
            window_apply_resize_delta,
            window_reveal_path,
            mpv_overlay_open_path,
            mpv_embed_play,
            mpv_embed_pause,
            mpv_embed_seek,
            mpv_embed_frame_step,
            mpv_embed_frame_back_step,
            mpv_embed_set_hwdec,
            mpv_embed_set_loop_file,
            mpv_embed_set_speed,
            mpv_embed_set_video_fill,
            mpv_embed_set_subtitle_delay,
            mpv_embed_select_track,
            mpv_embed_add_subtitle,
            mpv_embed_set_volume,
            mpv_embed_snapshot,
            mpv_embed_stop,
            media_files_from_paths,
            media_files_in_directory,
            startup_media_paths,
            platform_support,
            appearance_state,
            appearance_set_theme,
            appearance_set_accent_override,
            appearance_import_theme_plugin,
            appearance_set_plugin_enabled,
            appearance_reset,
            preferences_state,
            preferences_set_incognito_mode,
            preferences_set_quiet_keyboard_controls,
            preferences_set_language_mode,
            shell_preview_formats,
            shell_preview_open_default_apps_settings,
            shell_preview_register_formats,
            history_list,
            history_remember,
            history_resume_position,
            history_clear,
            playback_settings_state,
            playback_settings_update,
            playback_media_settings,
            playback_media_settings_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
