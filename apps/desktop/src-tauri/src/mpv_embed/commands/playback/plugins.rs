use super::super::super::*;

#[tauri::command]
pub async fn mpv_embed_set_plugin_property(
    app: AppHandle,
    property: String,
    value: Value,
) -> Result<MpvEmbedSnapshot, String> {
    let (property, value) = normalize_plugin_mpv_property(&property, &value)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            if plugin_subtitle_style_requires_ass_override(property) {
                player
                    .mpv
                    .set_property("sub-ass-override", "force")
                    .map_err(|error| format!("mpv subtitle style override failed: {error}"))?;
            }

            let targets = plugin_mpv_property_write_targets(property);
            let mut wrote_property = false;
            let mut first_error = None;
            for target in targets {
                match set_plugin_mpv_property_value(&player.mpv, target, &value) {
                    Ok(()) => wrote_property = true,
                    Err(error) => {
                        first_error.get_or_insert(error);
                    }
                }
            }
            if !wrote_property {
                let error = first_error.unwrap_or_else(|| "unknown error".to_string());
                return Err(format!("mpv plugin property failed: {error}"));
            }

            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}
