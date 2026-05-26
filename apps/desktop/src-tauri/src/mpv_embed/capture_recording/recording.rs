use super::super::*;

pub(in crate::mpv_embed) fn stop_recording_for_player(
    player: &mut MpvEmbedPlayer,
) -> Result<MpvRecordingState, String> {
    let Some(recording) = player.recording.take() else {
        return Ok(MpvRecordingState::inactive(None));
    };
    match recording.method {
        MpvRecordingMethod::StreamRecord => {
            player
                .mpv
                .set_property("stream-record", "")
                .map_err(|error| format!("mpv recording stop failed: {error}"))?;
        }
        MpvRecordingMethod::DumpCache { .. } => {
            let _ = player.mpv.command("dump-cache", &["0", "0", ""]);
        }
    }
    wait_for_recording_output(&recording.path, RECORDING_OUTPUT_READY_TIMEOUT)?;
    Ok(MpvRecordingState::inactive(Some(recording.path)))
}

pub(in crate::mpv_embed) fn recording_method_for_media_path(
    media_path: &str,
    start_position: f64,
) -> MpvRecordingMethod {
    if media_stream_scheme(media_path).is_some_and(is_live_recording_stream_scheme) {
        MpvRecordingMethod::StreamRecord
    } else {
        MpvRecordingMethod::DumpCache {
            start_position: recording_dump_start_position(start_position),
        }
    }
}

pub(in crate::mpv_embed) fn media_stream_scheme(media_path: &str) -> Option<&str> {
    media_path
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .filter(|scheme| !scheme.is_empty())
}

pub(in crate::mpv_embed) fn is_live_recording_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

pub(in crate::mpv_embed) fn recording_dump_start_position(position: f64) -> f64 {
    if !position.is_finite() {
        return 0.0;
    }
    (position - RECORDING_DUMP_PREROLL_SECONDS).max(0.0)
}

pub(in crate::mpv_embed) fn recording_time_arg(position: f64) -> Result<String, String> {
    if !position.is_finite() {
        return Err("recording start time is invalid".to_string());
    }
    Ok(format!("{:.3}", position.max(0.0)))
}

pub(in crate::mpv_embed) fn wait_for_recording_output(
    path: &str,
    timeout: Duration,
) -> Result<(), String> {
    let path = Path::new(path);
    let deadline = Instant::now() + timeout;
    loop {
        if recording_output_has_content(path).unwrap_or(false) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return ensure_recording_output_has_content(path);
        }
        thread::sleep(Duration::from_millis(40));
    }
}

pub(in crate::mpv_embed) fn recording_output_has_content(path: &Path) -> Result<bool, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("recording output was not created: {error}"))?;
    Ok(metadata.len() > 0)
}

pub(in crate::mpv_embed) fn ensure_recording_output_has_content(path: &Path) -> Result<(), String> {
    if !recording_output_has_content(path)? {
        let _ = fs::remove_file(path);
        return Err(
            "mpv produced an empty recording file; try recording for longer or using MKV"
                .to_string(),
        );
    }
    Ok(())
}
