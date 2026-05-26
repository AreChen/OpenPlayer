use super::super::super::*;

#[tauri::command]
pub async fn mpv_embed_play(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| {
        with_player(state, |player| {
            player.force_paused_until = None;
            player.ended = false;
            player
                .mpv
                .set_property("pause", false)
                .map_err(|error| format!("mpv play failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_pause(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| {
        with_player(state, |player| {
            player.force_paused_until = None;
            player
                .mpv
                .set_property("pause", true)
                .map_err(|error| format!("mpv pause failed: {error}"))?;
            Ok(player.snapshot(0, "paused"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_seek(app: AppHandle, position: f64) -> Result<MpvEmbedSnapshot, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("invalid mpv seek target".to_string());
    }

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player.force_paused_until = None;
            player.ended = false;
            player
                .mpv
                .command("seek", &[&position.to_string(), "absolute"])
                .map_err(|error| format!("mpv seek failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_frame_step(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| frame_step(state, "frame-step")).await
}

#[tauri::command]
pub async fn mpv_embed_frame_back_step(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| frame_step(state, "frame-back-step")).await
}
