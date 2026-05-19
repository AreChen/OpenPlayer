#[cfg(feature = "mpv-embed")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewWindow};
#[cfg(feature = "mpv-embed")]
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_runtime::ResizeDirection;
#[cfg(feature = "mpv-embed")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GWLP_HWNDPARENT, SetWindowLongPtrW};

#[cfg(feature = "mpv-embed")]
use tauri::WindowEvent;

#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;

#[cfg(feature = "mpv-embed")]
mod mpv_embed;

#[cfg(feature = "mpv-embed")]
use mpv_embed::{
    MpvEmbedSnapshot, MpvEmbedState, mpv_embed_pause, mpv_embed_play, mpv_embed_seek,
    mpv_embed_set_volume, mpv_embed_snapshot, mpv_embed_stop,
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
    sync_overlay_to_main(&app);
    Ok(())
}

#[tauri::command]
fn window_toggle_fullscreen(app: AppHandle) -> Result<(), String> {
    let main = main_window(&app)?;
    let is_fullscreen = main.is_fullscreen().map_err(|error| error.to_string())?;
    main.set_fullscreen(!is_fullscreen)
        .map_err(|error| error.to_string())?;
    sync_overlay_to_main(&app);
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_start_resize,
            window_close
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}

#[cfg(feature = "mpv-embed")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(MpvEmbedState::default())
        .setup(|app| {
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
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_close,
            window_start_drag,
            window_start_resize,
            mpv_overlay_open_path,
            mpv_embed_play,
            mpv_embed_pause,
            mpv_embed_seek,
            mpv_embed_set_volume,
            mpv_embed_snapshot,
            mpv_embed_stop
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
