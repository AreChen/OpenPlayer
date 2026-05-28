use super::super::*;

#[tauri::command]
pub async fn mpv_embed_capture_screenshot(
    app: AppHandle,
    format: Option<String>,
    directory: Option<String>,
) -> Result<MpvCaptureArtifact, String> {
    let capture_directory = capture_directory_for_app(&app, directory)?;
    let format = normalize_capture_image_format(format)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            fs::create_dir_all(&capture_directory)
                .map_err(|error| format!("failed to create capture directory: {error}"))?;
            let output_path = capture_output_path(
                &capture_directory,
                &player.path,
                current_time_ms_for_capture(),
                &format,
            );
            let output_text = output_path.to_string_lossy().to_string();
            player
                .mpv
                .command("screenshot-to-file", &[&output_text, "video"])
                .map_err(|error| format!("mpv screenshot failed: {error}"))?;
            let copied_to_clipboard = copy_image_file_to_clipboard(&output_path).is_ok();
            Ok(MpvCaptureArtifact {
                path: output_text,
                copied_to_clipboard,
            })
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_capture_plugin_frame(
    app: AppHandle,
    plugin_id: String,
    format: Option<String>,
    include_base64: Option<bool>,
) -> Result<MpvFrameCaptureArtifact, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
    let format = normalize_capture_image_format(format)?;
    let include_base64 = include_base64.unwrap_or(false);

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let output_path = plugin_frame_capture_output_path(
                &app_data_dir,
                &plugin_id,
                &player.path,
                current_time_ms_for_capture(),
                &format,
            )?;
            let parent = output_path
                .parent()
                .ok_or_else(|| "frame capture output path has no parent directory".to_string())?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create frame capture directory: {error}"))?;

            let output_text = output_path.to_string_lossy().to_string();
            player
                .mpv
                .command("screenshot-to-file", &[&output_text, "video"])
                .map_err(|error| format!("mpv frame capture failed: {error}"))?;
            frame_capture_artifact(&output_path, &format, include_base64)
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_recording_state(app: AppHandle) -> Result<MpvRecordingState, String> {
    let state = app.state::<MpvEmbedState>();
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;

    let Some(player) = player.as_mut() else {
        return Ok(MpvRecordingState::inactive(None));
    };
    player.drain_events();
    Ok(player.recording_state())
}

#[tauri::command]
pub async fn mpv_embed_start_recording(
    app: AppHandle,
    format: Option<String>,
    directory: Option<String>,
) -> Result<MpvRecordingState, String> {
    let recording_directory = recording_directory_for_app(&app, directory)?;
    let requested_format = normalize_recording_container_format(format)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            if player.recording.is_some() {
                return Ok(player.recording_state());
            }

            fs::create_dir_all(&recording_directory)
                .map_err(|error| format!("failed to create recording directory: {error}"))?;
            let start_position = player.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
            let method = recording_method_for_media_path(&player.path, start_position);
            let format = recording_container_format_for_method(&method, &requested_format);
            let output_path = recording_output_path(
                &recording_directory,
                &player.path,
                current_time_ms_for_capture(),
                &format,
            );
            let output_text = output_path.to_string_lossy().to_string();
            match &method {
                MpvRecordingMethod::StreamRecord => {
                    player
                        .mpv
                        .set_property("stream-record", output_text.as_str())
                        .map_err(|error| format!("mpv recording start failed: {error}"))?;
                }
                MpvRecordingMethod::DumpCache { start_position } => {
                    let start_arg = recording_time_arg(*start_position)?;
                    player
                        .mpv
                        .command("async", &["dump-cache", &start_arg, "no", &output_text])
                        .map_err(|error| format!("mpv recording start failed: {error}"))?;
                }
            }
            player.recording = Some(MpvRecordingSession {
                path: output_text,
                format,
                method,
            });
            Ok(player.recording_state())
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_stop_recording(app: AppHandle) -> Result<MpvRecordingState, String> {
    run_mpv_command(app, move |state| {
        with_player(state, stop_recording_for_player)
    })
    .await
}
