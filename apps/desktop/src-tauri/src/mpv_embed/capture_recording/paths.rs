use super::super::*;

pub(in crate::mpv_embed) fn capture_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> PathBuf {
    let stem = capture_file_stem(media_path);
    directory.join(format!("openplayer-{stem}-{timestamp_ms}.{format}"))
}

pub(in crate::mpv_embed) fn plugin_frame_capture_output_path(
    app_data_dir: &Path,
    plugin_id: &str,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> Result<PathBuf, String> {
    let plugin_directory = validate_frame_capture_plugin_id(plugin_id)?;
    let stem = capture_file_stem(media_path);
    Ok(app_data_dir
        .join("frame-captures")
        .join(plugin_directory)
        .join(format!("openplayer-{stem}-{timestamp_ms}.{format}")))
}

pub(in crate::mpv_embed) fn recording_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> PathBuf {
    let stem = capture_file_stem(media_path);
    directory.join(format!("openplayer-{stem}-{timestamp_ms}.{format}"))
}

pub(in crate::mpv_embed) fn capture_file_stem(media_path: &str) -> String {
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

pub(in crate::mpv_embed) fn current_time_ms_for_capture() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn validate_frame_capture_plugin_id(plugin_id: &str) -> Result<&str, String> {
    let plugin_id = plugin_id.trim();
    if plugin_id.len() > 128
        || !plugin_id.contains('.')
        || plugin_id.split('.').any(|segment| {
            segment.is_empty()
                || !segment
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_lowercase())
                || !segment.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
                })
        })
    {
        return Err("invalid frame capture plugin id".to_string());
    }
    Ok(plugin_id)
}
