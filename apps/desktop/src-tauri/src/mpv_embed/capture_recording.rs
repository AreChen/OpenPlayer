use super::*;

pub(super) fn capture_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create capture directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().picture_dir() {
        directory.push("OpenPlayer");
        directory.push("Captures");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve capture directory: {error}"))?;
    directory.push("captures");
    Ok(directory)
}

pub(super) fn recording_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create recording directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().video_dir() {
        directory.push("OpenPlayer");
        directory.push("Recordings");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve recording directory: {error}"))?;
    directory.push("recordings");
    Ok(directory)
}

pub(super) fn capture_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> PathBuf {
    let stem = capture_file_stem(media_path);
    directory.join(format!("openplayer-{stem}-{timestamp_ms}.{format}"))
}

pub(super) fn recording_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> PathBuf {
    let stem = capture_file_stem(media_path);
    directory.join(format!("openplayer-{stem}-{timestamp_ms}.{format}"))
}

pub(super) fn normalize_capture_image_format(format: Option<String>) -> Result<String, String> {
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("png")
        .to_ascii_lowercase();
    match format.as_str() {
        "png" | "jpg" | "webp" => Ok(format),
        "jpeg" => Ok("jpg".to_string()),
        _ => Err(format!("unsupported screenshot format: {format}")),
    }
}

pub(super) fn normalize_recording_container_format(
    format: Option<String>,
) -> Result<String, String> {
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    match format.as_str() {
        "mp4" | "mkv" | "ts" => Ok(format),
        _ => Err(format!("unsupported recording format: {format}")),
    }
}

pub(super) fn recording_container_format_for_method(
    method: &MpvRecordingMethod,
    requested_format: &str,
) -> String {
    match method {
        MpvRecordingMethod::DumpCache { .. } | MpvRecordingMethod::StreamRecord => {
            requested_format.to_string()
        }
    }
}

pub(super) fn normalize_capture_directory_override(
    directory: Option<String>,
) -> Result<Option<PathBuf>, String> {
    let Some(directory) = directory
        .as_deref()
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
    else {
        return Ok(None);
    };
    if directory.len() > 1024 {
        return Err("capture directory path is too long".to_string());
    }
    let path = PathBuf::from(directory);
    if !path.is_absolute() {
        return Err("capture directory path must be absolute".to_string());
    }
    if path.is_file() {
        return Err("capture directory path is not a directory".to_string());
    }
    Ok(Some(path))
}

pub(super) fn stop_recording_for_player(
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

pub(super) fn recording_method_for_media_path(
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

pub(super) fn media_stream_scheme(media_path: &str) -> Option<&str> {
    media_path
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .filter(|scheme| !scheme.is_empty())
}

pub(super) fn is_live_recording_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

pub(super) fn recording_dump_start_position(position: f64) -> f64 {
    if !position.is_finite() {
        return 0.0;
    }
    (position - RECORDING_DUMP_PREROLL_SECONDS).max(0.0)
}

pub(super) fn recording_time_arg(position: f64) -> Result<String, String> {
    if !position.is_finite() {
        return Err("recording start time is invalid".to_string());
    }
    Ok(format!("{:.3}", position.max(0.0)))
}

pub(super) fn wait_for_recording_output(path: &str, timeout: Duration) -> Result<(), String> {
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

pub(super) fn recording_output_has_content(path: &Path) -> Result<bool, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("recording output was not created: {error}"))?;
    Ok(metadata.len() > 0)
}

pub(super) fn ensure_recording_output_has_content(path: &Path) -> Result<(), String> {
    if !recording_output_has_content(path)? {
        let _ = fs::remove_file(path);
        return Err(
            "mpv produced an empty recording file; try recording for longer or using MKV"
                .to_string(),
        );
    }
    Ok(())
}

pub(super) fn copy_image_file_to_clipboard(path: &Path) -> Result<(), String> {
    let image = image::ImageReader::open(path)
        .map_err(|error| format!("failed to open screenshot for clipboard: {error}"))?
        .decode()
        .map_err(|error| format!("failed to decode screenshot for clipboard: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| format!("failed to access clipboard: {error}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(|error| format!("failed to copy screenshot to clipboard: {error}"))
}

pub(super) fn capture_file_stem(media_path: &str) -> String {
    let normalized = media_path.replace('\\', "/");
    let tail = normalized
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or("capture");
    let stem = tail
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(tail)
        .trim();
    let mut sanitized = String::new();
    for char in stem.chars() {
        if char.is_ascii_alphanumeric() || matches!(char, '-' | '_') {
            sanitized.push(char);
        } else if char.is_whitespace() || matches!(char, '.' | ':' | '/' | '\\') {
            sanitized.push('_');
        }
        if sanitized.len() >= 80 {
            break;
        }
    }
    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "capture".to_string()
    } else {
        sanitized
    }
}

pub(super) fn current_time_ms_for_capture() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
