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

#[tauri::command]
pub async fn mpv_embed_plugin_get_property(
    app: AppHandle,
    property: String,
) -> Result<Value, String> {
    let property = plugin_core_property_name(&property)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            read_plugin_core_property(&player.mpv, property)
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_set_property(
    app: AppHandle,
    property: String,
    value: Value,
) -> Result<MpvEmbedSnapshot, String> {
    let write = normalize_plugin_core_property(&property, &value)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            set_plugin_core_property_value(&player.mpv, write.property, &write.value)
                .map_err(|error| format!("mpv plugin core property failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_set_ab_loop(
    app: AppHandle,
    start: f64,
    end: f64,
) -> Result<MpvEmbedSnapshot, String> {
    let start = normalize_plugin_core_property("ab-loop-a", &Value::from(start))?;
    let end = normalize_plugin_core_property("ab-loop-b", &Value::from(end))?;
    if let (PluginMpvCoreValue::Number(start), PluginMpvCoreValue::Number(end)) =
        (&start.value, &end.value)
        && start >= end
    {
        return Err("plugin mpv AB loop start must be before end".to_string());
    }

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            set_plugin_core_property_value(&player.mpv, start.property, &start.value)
                .map_err(|error| format!("mpv plugin AB loop start failed: {error}"))?;
            set_plugin_core_property_value(&player.mpv, end.property, &end.value)
                .map_err(|error| format!("mpv plugin AB loop end failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_clear_ab_loop(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("ab-loop-a", "no")
                .map_err(|error| format!("mpv plugin AB loop clear start failed: {error}"))?;
            player
                .mpv
                .set_property("ab-loop-b", "no")
                .map_err(|error| format!("mpv plugin AB loop clear end failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_command(
    app: AppHandle,
    command: String,
    args: Value,
) -> Result<MpvEmbedSnapshot, String> {
    let command = normalize_plugin_core_command(&command, &args)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let args = command.args.iter().map(String::as_str).collect::<Vec<_>>();
            player
                .mpv
                .command(command.command, &args)
                .map_err(|error| format!("mpv plugin command failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_add_video_filter(
    app: AppHandle,
    plugin_id: String,
    filter_id: String,
    filter: String,
    params: Value,
) -> Result<MpvEmbedSnapshot, String> {
    let filter = normalize_plugin_video_filter(&plugin_id, &filter_id, &filter, &params)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("vf", &["add", filter.expression.as_str()])
                .map_err(|error| format!("mpv plugin video filter add failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_add_audio_filter(
    app: AppHandle,
    plugin_id: String,
    filter_id: String,
    filter: String,
    params: Value,
) -> Result<MpvEmbedSnapshot, String> {
    let filter = normalize_plugin_audio_filter(&plugin_id, &filter_id, &filter, &params)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("af", &["add", filter.expression.as_str()])
                .map_err(|error| format!("mpv plugin audio filter add failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_remove_audio_filter(
    app: AppHandle,
    plugin_id: String,
    filter_id: String,
) -> Result<MpvEmbedSnapshot, String> {
    let filter = plugin_audio_filter_remove_target(&plugin_id, &filter_id)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("af", &["remove", filter.as_str()])
                .map_err(|error| format!("mpv plugin audio filter remove failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_plugin_remove_video_filter(
    app: AppHandle,
    plugin_id: String,
    filter_id: String,
) -> Result<MpvEmbedSnapshot, String> {
    let filter = plugin_video_filter_remove_target(&plugin_id, &filter_id)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("vf", &["remove", filter.as_str()])
                .map_err(|error| format!("mpv plugin video filter remove failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}
