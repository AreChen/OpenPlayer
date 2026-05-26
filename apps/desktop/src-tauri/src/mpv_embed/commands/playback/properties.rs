use super::super::super::*;

#[tauri::command]
pub async fn mpv_embed_set_volume(app: AppHandle, volume: f64) -> Result<MpvEmbedSnapshot, String> {
    let volume = normalize_volume(volume)?;
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("volume", volume)
                .map_err(|error| format!("mpv volume failed: {error}"))?;
            player.volume = volume;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_speed(app: AppHandle, speed: f64) -> Result<MpvEmbedSnapshot, String> {
    let speed = normalize_playback_speed(speed)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("speed", speed)
                .map_err(|error| format!("mpv speed failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_hwdec(app: AppHandle, mode: String) -> Result<MpvEmbedSnapshot, String> {
    let hwdec = normalize_hwdec_mode(&mode)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("hwdec", hwdec)
                .map_err(|error| format!("mpv hardware decoding switch failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_video_fill(
    app: AppHandle,
    enabled: bool,
) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            set_video_fill_mode(&player.mpv, enabled)?;
            player.video_fill = enabled;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_loop_file(
    app: AppHandle,
    enabled: bool,
) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("loop-file", if enabled { "inf" } else { "no" })
                .map_err(|error| format!("mpv loop-file mode failed: {error}"))?;
            if enabled {
                player.ended = false;
            }
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_subtitle_delay(
    app: AppHandle,
    delay: f64,
) -> Result<MpvEmbedSnapshot, String> {
    let delay = normalize_subtitle_delay(delay)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("sub-delay", delay)
                .map_err(|error| format!("mpv subtitle delay failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}
