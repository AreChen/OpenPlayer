use super::super::*;

#[tauri::command]
#[allow(dead_code)]
pub fn mpv_embed_open_path(
    window: Window,
    state: State<'_, MpvEmbedState>,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    open_path_for_window(
        &window,
        state.inner(),
        path,
        resume_position,
        initial_volume,
        load_options,
    )
}

pub fn open_path_for_window(
    window: &impl HasWindowHandle,
    state: &MpvEmbedState,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    let path = validate_media_path(&path)?;
    let host = MpvVideoHost::new(window)?;
    let wid = host.wid();
    let mpv = create_embed_player(wid)?;
    #[cfg(target_os = "macos")]
    let render_context = create_macos_render_context(&mpv, &host)?;
    let path_text = path.to_string_lossy().to_string();
    let initial_volume = normalize_initial_volume(initial_volume)?;

    mpv.set_property("volume", initial_volume)
        .map_err(|error| format!("mpv initial volume failed: {error}"))?;
    configure_audio_visualizer(&mpv, &path);
    load_media_file_for_interactive_open(&mpv, &path_text, load_options.as_ref())?;
    load_sidecar_subtitles(&mpv, &path);

    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    if let Some(existing) = player.as_mut() {
        let _ = stop_recording_for_player(existing);
    }
    *player = Some(MpvEmbedPlayer {
        #[cfg(target_os = "macos")]
        _render_context: render_context,
        mpv,
        host,
        path: path_text,
        volume: initial_volume,
        video_fill: false,
        ended: false,
        force_paused_until: None,
        recording: None,
    });
    let next_player = player
        .as_mut()
        .ok_or_else(|| "mpv embed player initialization failed".to_string())?;
    next_player.apply_initial_resume_seek(resume_position);
    let snapshot = next_player.snapshot(wid, "playing");

    Ok(snapshot)
}

#[tauri::command]
pub fn mpv_embed_stop(window: Window, state: State<'_, MpvEmbedState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if MainThreadMarker::new().is_none() {
            let app = window.app_handle().clone();
            let app_for_stop = app.clone();
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            app.run_on_main_thread(move || {
                let state = app_for_stop.state::<MpvEmbedState>();
                let _ = sender.send(stop_player(state.inner()));
            })
            .map_err(|error| {
                format!("failed to schedule macOS mpv AppKit host teardown: {error}")
            })?;

            return receiver.recv().map_err(|_| {
                "macOS mpv AppKit host teardown did not return a result".to_string()
            })?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = window;

    stop_player(state.inner())
}
