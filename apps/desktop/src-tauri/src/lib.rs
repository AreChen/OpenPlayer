use tauri::Window;

#[cfg(feature = "mpv-embed")]
use tauri::Manager;

#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;

#[cfg(feature = "mpv-embed")]
mod mpv_embed;

#[cfg(feature = "mpv-embed")]
use mpv_embed::{MpvEmbedState, mpv_embed_open_path, mpv_embed_stop};

#[cfg(feature = "mpv-smoke")]
pub use mpv_smoke::{MpvSmokeReport, create_headless_probe};

#[tauri::command]
fn window_minimize(window: Window) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

#[tauri::command]
fn window_toggle_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|error| error.to_string())? {
        window.unmaximize().map_err(|error| error.to_string())
    } else {
        window.maximize().map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn window_close(window: Window) -> Result<(), String> {
    window.close().map_err(|error| error.to_string())
}

#[cfg(not(feature = "mpv-embed"))]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_close
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}

#[cfg(feature = "mpv-embed")]
pub fn run() {
    tauri::Builder::default()
        .manage(MpvEmbedState::default())
        .setup(|app| {
            if let Ok(path) = std::env::var("OPENPLAYER_MPV_EMBED_FILE") {
                if let Some(window) = app.get_webview_window("main") {
                    let state = app.state::<MpvEmbedState>();
                    if let Err(error) =
                        mpv_embed::open_path_for_window(&window, state.inner(), path)
                    {
                        eprintln!("startup mpv embed failed: {error}");
                    }
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_close,
            mpv_embed_open_path,
            mpv_embed_stop
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
