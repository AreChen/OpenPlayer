use super::*;

pub(super) fn normalize_playback_speed(speed: f64) -> Result<f64, String> {
    if !speed.is_finite() {
        return Err("invalid mpv playback speed".to_string());
    }

    Ok(speed.clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED))
}

pub(super) fn normalize_subtitle_delay(delay: f64) -> Result<f64, String> {
    if !delay.is_finite() {
        return Err("invalid mpv subtitle delay".to_string());
    }

    Ok(delay.clamp(MIN_SUBTITLE_DELAY, MAX_SUBTITLE_DELAY))
}

pub(super) fn set_video_fill_mode(mpv: &libmpv2::Mpv, enabled: bool) -> Result<(), String> {
    let panscan = if enabled { 1.0 } else { 0.0 };
    mpv.set_property("panscan", panscan)
        .map_err(|error| format!("mpv video layout failed: {error}"))
}

pub(super) fn normalize_hwdec_mode(mode: &str) -> Result<&'static str, String> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "hardware" | "auto" | "auto-safe" => Ok("auto-safe"),
        "software" | "no" | "off" => Ok("no"),
        _ => Err("invalid mpv hardware decoding mode".to_string()),
    }
}

pub(super) fn track_property_for_kind(kind: &str) -> Result<&'static str, String> {
    match kind {
        "audio" => Ok("aid"),
        "video" => Ok("vid"),
        "subtitle" | "sub" => Ok("sid"),
        _ => Err("invalid mpv track kind".to_string()),
    }
}

pub(super) fn normalize_initial_resume_position(position: Option<f64>) -> Option<f64> {
    position.filter(|position| position.is_finite() && *position > 0.0)
}

pub(super) fn normalize_volume(volume: f64) -> Result<f64, String> {
    if !volume.is_finite() {
        return Err("invalid mpv volume".to_string());
    }

    Ok(volume.clamp(0.0, 100.0))
}

pub(super) fn normalize_initial_volume(volume: Option<f64>) -> Result<f64, String> {
    volume.map_or(Ok(DEFAULT_VOLUME), normalize_volume)
}
