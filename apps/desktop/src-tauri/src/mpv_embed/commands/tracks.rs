use super::super::*;

#[tauri::command]
pub async fn mpv_embed_select_track(
    app: AppHandle,
    kind: String,
    track_id: Option<i64>,
) -> Result<MpvEmbedSnapshot, String> {
    let property = track_property_for_kind(&kind)?;
    if track_id.is_some_and(|id| id <= 0) {
        return Err("invalid mpv track id".to_string());
    }

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            if let Some(id) = track_id {
                player
                    .mpv
                    .set_property(property, id)
                    .map_err(|error| format!("mpv track selection failed: {error}"))?;
            } else {
                player
                    .mpv
                    .set_property(property, "no")
                    .map_err(|error| format!("mpv track disable failed: {error}"))?;
            }
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_add_subtitle(
    app: AppHandle,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    let path = validate_subtitle_path(&path)?;
    let path_text = path.to_string_lossy().to_string();

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("sub-add", &[&path_text, "select"])
                .map_err(|error| format!("mpv subtitle load failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}
