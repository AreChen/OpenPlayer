#[cfg(feature = "mpv-embed")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{collections::HashMap, sync::Mutex, thread, time::Duration};
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
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size, State, WebviewWindow,
};
#[cfg(feature = "mpv-embed")]
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_runtime::ResizeDirection;
#[cfg(feature = "mpv-embed")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GWLP_HWNDPARENT, SetWindowLongPtrW};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_LWIN,
            VK_MENU, VK_OEM_COMMA, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_UP,
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

mod playback_store;

#[cfg(feature = "mpv-embed")]
use mpv_embed::{
    MpvEmbedSnapshot, MpvEmbedState, mpv_embed_add_subtitle, mpv_embed_frame_back_step,
    mpv_embed_frame_step, mpv_embed_pause, mpv_embed_play, mpv_embed_seek, mpv_embed_select_track,
    mpv_embed_set_speed, mpv_embed_set_subtitle_delay, mpv_embed_set_volume, mpv_embed_snapshot,
    mpv_embed_stop,
};
use playback_store::{PlaybackStoreState, history_list, history_remember, history_resume_position};

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
}

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

        if let Some(state) = NATIVE_SHORTCUT_STATE.get() {
            if let Ok(mut stored_hook) = state.hook.lock() {
                *stored_hook = Some(hook as usize);
            }
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
    if is_key_up {
        if let Ok(mut pressed_keys) = state.pressed_keys.lock() {
            pressed_keys.remove(&key.vkCode);
        }
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
    let is_fullscreen = main.is_fullscreen().map_err(|error| error.to_string())?;
    if is_fullscreen {
        let placement = window_state
            .fullscreen_restore
            .lock()
            .map_err(|_| "window state lock failed".to_string())?
            .clone();

        if let Some(placement) = placement {
            restore_window_after_fullscreen(&main, placement)?;
            *window_state
                .fullscreen_restore
                .lock()
                .map_err(|_| "window state lock failed".to_string())? = None;
        } else {
            main.set_fullscreen(false)
                .map_err(|error| error.to_string())?;
        }
    } else {
        let placement = capture_window_placement(&main)?;
        main.set_fullscreen(true)
            .map_err(|error| error.to_string())?;
        *window_state
            .fullscreen_restore
            .lock()
            .map_err(|_| "window state lock failed".to_string())? = Some(placement);
    }

    schedule_overlay_sync_to_main(&app);
    Ok(())
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

fn sync_overlay_to_main(app: &AppHandle) {
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
    focus_overlay_window(app);
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

fn capture_window_placement(window: &WebviewWindow) -> Result<WindowPlacement, String> {
    Ok(WindowPlacement {
        position: window.outer_position().map_err(|error| error.to_string())?,
        size: window.outer_size().map_err(|error| error.to_string())?,
        maximized: window.is_maximized().map_err(|error| error.to_string())?,
    })
}

fn restore_window_after_fullscreen(
    window: &WebviewWindow,
    placement: WindowPlacement,
) -> Result<(), String> {
    window
        .set_fullscreen(false)
        .map_err(|error| error.to_string())?;

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

#[cfg(feature = "mpv-embed")]
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

#[cfg(feature = "mpv-embed")]
fn window_hwnd(window: &impl HasWindowHandle) -> Result<isize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        _ => Err("window operation is only wired for Windows HWND targets".to_string()),
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
    let direction = match direction.as_str() {
        "East" => ResizeDirection::East,
        "North" => ResizeDirection::North,
        "NorthEast" => ResizeDirection::NorthEast,
        "NorthWest" => ResizeDirection::NorthWest,
        "South" => ResizeDirection::South,
        "SouthEast" => ResizeDirection::SouthEast,
        "SouthWest" => ResizeDirection::SouthWest,
        "West" => ResizeDirection::West,
        _ => return Err(format!("invalid resize direction: {direction}")),
    };

    main_window(&app)?
        .as_ref()
        .window()
        .start_resize_dragging(direction)
        .map_err(|error| error.to_string())
}

#[cfg(feature = "mpv-embed")]
#[tauri::command]
fn mpv_overlay_open_path(
    app: AppHandle,
    state: tauri::State<'_, MpvEmbedState>,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    let main = main_window(&app)?;
    sync_overlay_to_main(&app);
    mpv_embed::open_path_for_window(&main, state.inner(), path)
}

#[cfg(not(feature = "mpv-embed"))]
pub fn run() {
    tauri::Builder::default()
        .manage(WindowState::default())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(PlaybackStoreState::open(app.handle()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_update_shortcuts,
            window_set_shortcuts_enabled,
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_focus_overlay,
            window_start_resize,
            window_close,
            history_list,
            history_remember,
            history_resume_position
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}

#[cfg(feature = "mpv-embed")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(WindowState::default())
        .manage(MpvEmbedState::default())
        .setup(|app| {
            app.manage(PlaybackStoreState::open(app.handle()));
            install_native_shortcut_hook(app.handle().clone());
            if let Some(window) = app.get_webview_window("main") {
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
                .visible(false)
                .build()
                .map_err(|error| format!("failed to create overlay controls window: {error}"))?;
                set_overlay_owner(&window, &overlay);

                let app_handle = app.handle().clone();
                sync_overlay_to_main(&app_handle);
                let _ = overlay.show();
                window.on_window_event(move |event| {
                    if matches!(
                        event,
                        WindowEvent::Moved(_)
                            | WindowEvent::Resized(_)
                            | WindowEvent::ScaleFactorChanged { .. }
                    ) {
                        sync_overlay_to_main(&app_handle);
                        let state = app_handle.state::<MpvEmbedState>();
                        let _ = state.resize_video_host();
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
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_close,
            window_focus_overlay,
            window_start_drag,
            window_start_resize,
            mpv_overlay_open_path,
            mpv_embed_play,
            mpv_embed_pause,
            mpv_embed_seek,
            mpv_embed_frame_step,
            mpv_embed_frame_back_step,
            mpv_embed_set_speed,
            mpv_embed_set_subtitle_delay,
            mpv_embed_select_track,
            mpv_embed_add_subtitle,
            mpv_embed_set_volume,
            mpv_embed_snapshot,
            mpv_embed_stop,
            history_list,
            history_remember,
            history_resume_position
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
