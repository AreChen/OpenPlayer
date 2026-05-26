use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri::Manager;

use crate::mpv_embed::{self, MpvEmbedSnapshot, MpvEmbedState, MpvLoadOptions};

use super::{main_window, overlay};

pub(super) fn open_path(
    app: AppHandle,
    state: &MpvEmbedState,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = state;
        return open_path_for_main_window_on_main_thread(
            app,
            path,
            resume_position,
            initial_volume,
            load_options,
        );
    }

    #[cfg(not(target_os = "macos"))]
    {
        let main = main_window(&app)?;
        overlay::sync_overlay_to_main(&app);
        mpv_embed::open_path_for_window(
            &main,
            state,
            path,
            resume_position,
            initial_volume,
            load_options,
        )
    }
}

#[cfg(all(feature = "mpv-embed", target_os = "macos"))]
fn open_path_for_main_window_on_main_thread(
    app: AppHandle,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    if objc2::MainThreadMarker::new().is_some() {
        return open_path_for_main_window_now(
            &app,
            path,
            resume_position,
            initial_volume,
            load_options,
        );
    }

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let app_for_open = app.clone();
    app.run_on_main_thread(move || {
        let result = open_path_for_main_window_now(
            &app_for_open,
            path,
            resume_position,
            initial_volume,
            load_options,
        );
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
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    let main = main_window(app)?;
    overlay::sync_overlay_to_main(app);
    let state = app.state::<MpvEmbedState>();
    mpv_embed::open_path_for_window(
        &main,
        state.inner(),
        path,
        resume_position,
        initial_volume,
        load_options,
    )
}
