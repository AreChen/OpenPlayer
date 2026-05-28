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

#[tauri::command]
pub async fn mpv_embed_current_subtitle_cue(
    app: AppHandle,
) -> Result<Option<CurrentSubtitleCue>, String> {
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let Some(track) = read_tracks(&player.mpv)
                .into_iter()
                .find(|track| track.kind == "sub" && track.selected)
            else {
                return Ok(None);
            };
            let Some(text) = read_current_subtitle_text(&player.mpv) else {
                return Ok(None);
            };

            Ok(Some(CurrentSubtitleCue {
                track_id: track.id,
                title: track.title,
                language: track.language,
                start: read_current_subtitle_time(&player.mpv, "sub-start"),
                end: read_current_subtitle_time(&player.mpv, "sub-end"),
                text,
            }))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_load_generated_subtitle(
    app: AppHandle,
    plugin_id: String,
    name: Option<String>,
    format: String,
    content: String,
    select: Option<bool>,
) -> Result<GeneratedSubtitleLoadResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
    let path = write_generated_subtitle_file(
        &app_data_dir,
        &plugin_id,
        name.as_deref(),
        &format,
        &content,
    )?;
    let result_path = path.to_string_lossy().to_string();
    let command_path = result_path.clone();
    let mode = if select.unwrap_or(true) {
        "select".to_string()
    } else {
        "auto".to_string()
    };

    let snapshot = run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("sub-add", &[command_path.as_str(), mode.as_str()])
                .map_err(|error| format!("mpv generated subtitle load failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await?;

    Ok(GeneratedSubtitleLoadResult {
        path: result_path,
        snapshot,
    })
}

#[tauri::command]
pub async fn mpv_embed_load_generated_subtitle_cues(
    app: AppHandle,
    plugin_id: String,
    name: Option<String>,
    format: String,
    cues: Vec<GeneratedSubtitleCue>,
    select: Option<bool>,
) -> Result<GeneratedSubtitleLoadResult, String> {
    let content = format_generated_subtitle_cues(&format, &cues)?;
    mpv_embed_load_generated_subtitle(app, plugin_id, name, format, content, select).await
}

#[tauri::command]
pub async fn mpv_embed_list_generated_subtitles(
    app: AppHandle,
    plugin_id: String,
) -> Result<Vec<GeneratedSubtitleTrack>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            Ok(read_plugin_generated_subtitle_tracks(
                &player.mpv,
                &app_data_dir,
                &plugin_id,
            ))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_remove_generated_subtitle(
    app: AppHandle,
    plugin_id: String,
    track_id: i64,
) -> Result<MpvEmbedSnapshot, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;

    let (snapshot, removed_path) = run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let managed_path = plugin_generated_subtitle_track_path(
                &player.mpv,
                &app_data_dir,
                &plugin_id,
                track_id,
            )?;
            let track_arg = track_id.to_string();
            player
                .mpv
                .command("sub-remove", &[track_arg.as_str()])
                .map_err(|error| format!("mpv generated subtitle remove failed: {error}"))?;
            Ok((player.snapshot(0, "playing"), managed_path))
        })
    })
    .await?;
    let _ = fs::remove_file(removed_path);
    Ok(snapshot)
}

#[tauri::command]
pub async fn mpv_embed_replace_generated_subtitle(
    app: AppHandle,
    plugin_id: String,
    track_id: i64,
    name: Option<String>,
    format: String,
    content: String,
    select: Option<bool>,
) -> Result<GeneratedSubtitleLoadResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
    let path = write_generated_subtitle_file(
        &app_data_dir,
        &plugin_id,
        name.as_deref(),
        &format,
        &content,
    )?;
    let result_path = path.to_string_lossy().to_string();
    let command_path = result_path.clone();
    let mode = if select.unwrap_or(true) {
        "select".to_string()
    } else {
        "auto".to_string()
    };

    let (snapshot, removed_path) = run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let managed_path = plugin_generated_subtitle_track_path(
                &player.mpv,
                &app_data_dir,
                &plugin_id,
                track_id,
            )?;
            let track_arg = track_id.to_string();
            player
                .mpv
                .command("sub-remove", &[track_arg.as_str()])
                .map_err(|error| {
                    format!("mpv generated subtitle replace remove failed: {error}")
                })?;
            player
                .mpv
                .command("sub-add", &[command_path.as_str(), mode.as_str()])
                .map_err(|error| format!("mpv generated subtitle replace load failed: {error}"))?;
            Ok((player.snapshot(0, "playing"), managed_path))
        })
    })
    .await?;
    let _ = fs::remove_file(removed_path);

    Ok(GeneratedSubtitleLoadResult {
        path: result_path,
        snapshot,
    })
}

#[tauri::command]
pub async fn mpv_embed_replace_generated_subtitle_cues(
    app: AppHandle,
    plugin_id: String,
    track_id: i64,
    name: Option<String>,
    format: String,
    cues: Vec<GeneratedSubtitleCue>,
    select: Option<bool>,
) -> Result<GeneratedSubtitleLoadResult, String> {
    let content = format_generated_subtitle_cues(&format, &cues)?;
    mpv_embed_replace_generated_subtitle(app, plugin_id, track_id, name, format, content, select)
        .await
}

#[tauri::command]
pub async fn mpv_embed_append_generated_subtitle_cues(
    app: AppHandle,
    plugin_id: String,
    track_id: i64,
    cues: Vec<GeneratedSubtitleCue>,
    select: Option<bool>,
) -> Result<GeneratedSubtitleLoadResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;

    let managed_path = run_mpv_command(app.clone(), {
        let plugin_id = plugin_id.clone();
        move |state| {
            with_player(state, |player| {
                plugin_generated_subtitle_track_path(
                    &player.mpv,
                    &app_data_dir,
                    &plugin_id,
                    track_id,
                )
            })
        }
    })
    .await?;

    append_generated_subtitle_cues_file(&managed_path, &cues)?;
    let result_path = managed_path.to_string_lossy().to_string();
    let snapshot = run_mpv_command(app, move |state| {
        with_player(state, |player| {
            let track_arg = track_id.to_string();
            player
                .mpv
                .command("sub-reload", &[track_arg.as_str()])
                .map_err(|error| format!("mpv generated subtitle append reload failed: {error}"))?;
            if select.unwrap_or(true) {
                player.mpv.set_property("sid", track_id).map_err(|error| {
                    format!("mpv generated subtitle append select failed: {error}")
                })?;
            }
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await?;

    Ok(GeneratedSubtitleLoadResult {
        path: result_path,
        snapshot,
    })
}

fn read_plugin_generated_subtitle_tracks(
    mpv: &libmpv2::Mpv,
    app_data_dir: &Path,
    plugin_id: &str,
) -> Vec<GeneratedSubtitleTrack> {
    let count = mpv
        .get_property::<i64>("track-list/count")
        .unwrap_or(0)
        .clamp(0, MAX_TRACKS);
    let mut tracks = Vec::new();

    for index in 0..count {
        if let Some(track) = plugin_generated_subtitle_track_at(mpv, app_data_dir, plugin_id, index)
        {
            tracks.push(track);
        }
    }

    tracks
}

fn plugin_generated_subtitle_track_path(
    mpv: &libmpv2::Mpv,
    app_data_dir: &Path,
    plugin_id: &str,
    track_id: i64,
) -> Result<PathBuf, String> {
    if track_id <= 0 {
        return Err("invalid generated subtitle track id".to_string());
    }
    let count = mpv
        .get_property::<i64>("track-list/count")
        .unwrap_or(0)
        .clamp(0, MAX_TRACKS);

    for index in 0..count {
        let Ok(id) = mpv.get_property::<i64>(&format!("track-list/{index}/id")) else {
            continue;
        };
        if id != track_id {
            continue;
        }
        if !is_generated_subtitle_track(mpv, index) {
            break;
        }
        let path = read_generated_subtitle_track_path(mpv, index)
            .ok_or_else(|| "generated subtitle track has no managed path".to_string())?;
        return plugin_generated_subtitle_path(app_data_dir, plugin_id, &path);
    }

    Err("generated subtitle track is not owned by the current plugin".to_string())
}

fn plugin_generated_subtitle_track_at(
    mpv: &libmpv2::Mpv,
    app_data_dir: &Path,
    plugin_id: &str,
    index: i64,
) -> Option<GeneratedSubtitleTrack> {
    if !is_generated_subtitle_track(mpv, index) {
        return None;
    }
    let id = mpv
        .get_property::<i64>(&format!("track-list/{index}/id"))
        .ok()
        .filter(|id| *id > 0)?;
    let path = read_generated_subtitle_track_path(mpv, index)?;
    let managed_path = plugin_generated_subtitle_path(app_data_dir, plugin_id, &path).ok()?;

    Some(GeneratedSubtitleTrack {
        id,
        title: read_optional_string(mpv, &format!("track-list/{index}/title")),
        language: read_optional_string(mpv, &format!("track-list/{index}/lang")),
        codec: read_optional_string(mpv, &format!("track-list/{index}/codec")),
        selected: mpv
            .get_property::<bool>(&format!("track-list/{index}/selected"))
            .unwrap_or(false),
        path: managed_path.to_string_lossy().to_string(),
    })
}

fn is_generated_subtitle_track(mpv: &libmpv2::Mpv, index: i64) -> bool {
    mpv.get_property::<String>(&format!("track-list/{index}/type"))
        .is_ok_and(|kind| kind == "sub")
        && mpv
            .get_property::<bool>(&format!("track-list/{index}/external"))
            .unwrap_or(false)
}

fn read_generated_subtitle_track_path(mpv: &libmpv2::Mpv, index: i64) -> Option<String> {
    read_optional_string(mpv, &format!("track-list/{index}/external-filename"))
        .or_else(|| read_optional_string(mpv, &format!("track-list/{index}/filename")))
}

fn read_current_subtitle_text(mpv: &libmpv2::Mpv) -> Option<String> {
    mpv.get_property::<String>("sub-text")
        .ok()
        .map(|text| text.replace("\r\n", "\n").replace('\r', "\n"))
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn read_current_subtitle_time(mpv: &libmpv2::Mpv, property: &str) -> Option<f64> {
    mpv.get_property::<f64>(property)
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}
