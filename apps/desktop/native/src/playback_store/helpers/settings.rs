use super::super::*;

pub(in crate::playback_store) fn merge_settings_update(
    settings: &mut PlaybackSettings,
    update: PlaybackSettingsUpdate,
) {
    if let Some(volume) = update.volume {
        settings.volume = normalize_volume(volume);
    }
    if let Some(loop_mode) = update.loop_mode {
        settings.loop_mode = normalize_loop_mode(&loop_mode);
    }
    if let Some(hwdec_mode) = update.hwdec_mode {
        settings.hwdec_mode = normalize_hwdec_mode(&hwdec_mode);
    }
    if let Some(playback_speed) = update.playback_speed {
        settings.playback_speed = normalize_playback_speed(playback_speed);
    }
    if let Some(video_fill) = update.video_fill {
        settings.video_fill = video_fill;
    }
    if let Some(time_display_mode) = update.time_display_mode {
        settings.time_display_mode = normalize_time_display_mode(&time_display_mode);
    }
}

pub(in crate::playback_store) fn sanitize_playback_settings(
    mut settings: PlaybackSettings,
) -> PlaybackSettings {
    settings.volume = normalize_volume(settings.volume);
    settings.loop_mode = normalize_loop_mode(&settings.loop_mode);
    settings.hwdec_mode = normalize_hwdec_mode(&settings.hwdec_mode);
    settings.playback_speed = normalize_playback_speed(settings.playback_speed);
    settings.time_display_mode = normalize_time_display_mode(&settings.time_display_mode);
    settings
}

pub(in crate::playback_store) fn sanitize_media_settings(
    mut settings: MediaPlaybackSettings,
) -> MediaPlaybackSettings {
    if let Some(id) = settings.subtitle_track_id
        && id <= 0
    {
        settings.subtitle_track_id = None;
    }
    settings
}

pub(in crate::playback_store) fn normalize_volume(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        DEFAULT_VOLUME
    }
}

pub(in crate::playback_store) fn normalize_loop_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "one" => "one".to_string(),
        "all" => "all".to_string(),
        _ => DEFAULT_LOOP_MODE.to_string(),
    }
}

pub(in crate::playback_store) fn normalize_hwdec_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "software" => "software".to_string(),
        _ => DEFAULT_HWDEC_MODE.to_string(),
    }
}

pub(in crate::playback_store) fn normalize_playback_speed(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED)
    } else {
        DEFAULT_PLAYBACK_SPEED
    }
}

pub(in crate::playback_store) fn normalize_time_display_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "frames" => "frames".to_string(),
        _ => DEFAULT_TIME_DISPLAY_MODE.to_string(),
    }
}

pub(in crate::playback_store) fn normalize_track_id(
    track_id: Option<i64>,
) -> Result<Option<i64>, String> {
    match track_id {
        Some(id) if id <= 0 => Err("invalid media playback track id".to_string()),
        other => Ok(other),
    }
}
