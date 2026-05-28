use super::super::*;

#[tauri::command]
pub async fn mpv_embed_extract_audio_clip(
    app: AppHandle,
    plugin_id: String,
    start: Option<f64>,
    duration: Option<f64>,
    sample_rate: Option<u32>,
    channels: Option<String>,
    include_base64: Option<bool>,
) -> Result<MpvAudioClipArtifact, String> {
    let mut request = normalize_audio_clip_extract_request(
        start,
        duration,
        sample_rate,
        channels,
        include_base64.unwrap_or(false),
    )?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;

    let (media_path, current_position) = {
        let state = app.state::<MpvEmbedState>();
        let mut player = state
            .player
            .lock()
            .map_err(|_| "mpv embed state lock failed".to_string())?;
        let player = player
            .as_mut()
            .ok_or_else(|| "audio.extractClip requires loaded media".to_string())?;
        player.drain_events();
        (
            player.path.clone(),
            player.mpv.get_property::<f64>("time-pos").unwrap_or(0.0),
        )
    };
    if start.is_none() {
        request.start = ((current_position.max(0.0)) * 1000.0).round() / 1000.0;
    }

    let output_path = audio_clip_output_path(
        &app_data_dir,
        &plugin_id,
        &media_path,
        current_time_ms_for_capture(),
    )?;
    tauri::async_runtime::spawn_blocking(move || {
        export_audio_clip_to_wav(&media_path, &output_path, &request)
    })
    .await
    .map_err(|error| format!("audio clip extraction task failed: {error}"))?
}
