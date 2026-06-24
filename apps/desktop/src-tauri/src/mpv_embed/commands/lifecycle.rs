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
    let path_text = path.to_string_lossy().to_string();
    let initial_volume = normalize_initial_volume(initial_volume)?;
    stop_existing_player_for_replacement(state)?;

    let host = MpvVideoHost::new(window)?;
    let wid = host.wid();
    let mpv = create_embed_player(wid)?;
    #[cfg(target_os = "macos")]
    let render_context = create_macos_render_context(&mpv, &host)?;

    mpv.set_property("volume", initial_volume)
        .map_err(|error| format!("mpv initial volume failed: {error}"))?;
    configure_audio_visualizer(&mpv, &path);
    load_media_file_for_interactive_open(&mpv, &path_text, load_options.as_ref())?;
    load_sidecar_subtitles(&mpv, &path);
    let opening = is_network_stream_media_url(&path_text);

    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    *player = Some(MpvEmbedPlayer {
        #[cfg(target_os = "macos")]
        _render_context: render_context,
        mpv,
        host,
        path: path_text,
        volume: initial_volume,
        video_fill: false,
        opening,
        ended: false,
        force_paused_until: None,
        recording: None,
    });
    let next_player = player
        .as_mut()
        .ok_or_else(|| "mpv embed player initialization failed".to_string())?;
    let snapshot = if opening {
        startup_snapshot_for_interactive_open(
            &next_player.path,
            wid,
            initial_volume,
            false,
            "playing",
        )
    } else {
        next_player.apply_initial_resume_seek(resume_position);
        next_player.snapshot(wid, "playing")
    };

    Ok(snapshot)
}

pub(crate) fn stop_embedded_player_for_close(app: &AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if MainThreadMarker::new().is_none() {
            return stop_player_on_macos_main_thread(app);
        }
    }

    let state = app.state::<MpvEmbedState>();
    stop_player(state.inner())
}

#[cfg(target_os = "macos")]
fn stop_player_on_macos_main_thread(app: &AppHandle) -> Result<(), String> {
    let app_for_stop = app.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let state = app_for_stop.state::<MpvEmbedState>();
        let _ = sender.send(stop_player(state.inner()));
    })
    .map_err(|error| format!("failed to schedule macOS mpv AppKit host teardown: {error}"))?;

    receiver
        .recv()
        .map_err(|_| "macOS mpv AppKit host teardown did not return a result".to_string())?
}

#[tauri::command]
pub fn mpv_embed_stop(window: Window, state: State<'_, MpvEmbedState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if MainThreadMarker::new().is_none() {
            return stop_player_on_macos_main_thread(window.app_handle());
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = window;

    stop_player(state.inner())
}
