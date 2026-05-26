use super::super::super::*;

#[tauri::command]
pub async fn mpv_embed_snapshot(app: AppHandle) -> Result<Option<MpvEmbedSnapshot>, String> {
    run_mpv_command(app, |state| {
        let mut player = state
            .player
            .lock()
            .map_err(|_| "mpv embed state lock failed".to_string())?;

        Ok(player.as_mut().map(|player| player.snapshot(0, "playing")))
    })
    .await
}
