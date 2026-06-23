use super::super::*;

#[tauri::command]
pub async fn mpv_embed_export_media_segment(
    app: AppHandle,
    kind: Option<String>,
    format: Option<String>,
    start: Option<f64>,
    duration: Option<f64>,
    file_name: Option<String>,
    directory: Option<String>,
) -> Result<MpvMediaSegmentExportArtifact, String> {
    let request = normalize_media_segment_export_request(kind, format, start, duration, file_name)?;
    let export_directory = media_segment_export_directory_for_app(&app, directory)?;

    let media_path = {
        let state = app.state::<MpvEmbedState>();
        let mut player = state
            .player
            .lock()
            .map_err(|_| "mpv embed state lock failed".to_string())?;
        let player = player
            .as_mut()
            .ok_or_else(|| "media.exportSegment requires loaded media".to_string())?;
        player.drain_events();
        player.path.clone()
    };

    let output_path = media_segment_export_output_path(
        &export_directory,
        &media_path,
        current_time_ms_for_capture(),
        &request,
    );
    tauri::async_runtime::spawn_blocking(move || {
        export_media_segment_to_file(&media_path, &output_path, &request)
    })
    .await
    .map_err(|error| format!("media segment export task failed: {error}"))?
}
