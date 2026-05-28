use super::*;

pub(in crate::mpv_embed) const MAX_GENERATED_SUBTITLE_BYTES: usize = 2 * 1024 * 1024;
const GENERATED_SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub"];

pub(in crate::mpv_embed) fn validate_subtitle_path(path: &str) -> Result<PathBuf, String> {
    let path = validate_media_path(path)?;
    if is_supported_subtitle_path(&path) {
        Ok(path)
    } else {
        Err(format!("unsupported subtitle file: {}", path.display()))
    }
}

fn is_supported_subtitle_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_SUBTITLE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

pub(in crate::mpv_embed) fn discover_sidecar_subtitles(media_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = media_path.parent() else {
        return Vec::new();
    };
    let Some(media_stem) = media_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    let mut subtitles: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| is_supported_subtitle_path(path))
        .filter(|path| is_matching_sidecar_stem(path, media_stem))
        .collect();

    subtitles.sort_by(|left, right| {
        sidecar_sort_key(left, media_stem).cmp(&sidecar_sort_key(right, media_stem))
    });
    subtitles
}

pub(in crate::mpv_embed) fn write_generated_subtitle_file(
    app_data_dir: &Path,
    plugin_id: &str,
    name: Option<&str>,
    format: &str,
    content: &str,
) -> Result<PathBuf, String> {
    let plugin_directory = validate_generated_subtitle_plugin_id(plugin_id)?;
    let extension = normalize_generated_subtitle_format(format)?;
    let byte_len = content.len();
    if content.trim().is_empty() {
        return Err("generated subtitle content cannot be empty".to_string());
    }
    if byte_len > MAX_GENERATED_SUBTITLE_BYTES {
        return Err(format!(
            "generated subtitle content is too large: {byte_len} bytes exceeds {MAX_GENERATED_SUBTITLE_BYTES}"
        ));
    }

    let safe_name = sanitize_generated_subtitle_name(name.unwrap_or("generated-subtitle"));
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before unix epoch: {error}"))?
        .as_millis();
    let directory = generated_subtitle_directory(app_data_dir, plugin_directory)?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create generated subtitle directory: {error}"))?;
    let path = directory.join(format!("{timestamp_ms}-{safe_name}.{extension}"));
    fs::write(&path, content.as_bytes())
        .map_err(|error| format!("failed to write generated subtitle file: {error}"))?;
    Ok(path)
}

pub(in crate::mpv_embed) fn generated_subtitle_directory(
    app_data_dir: &Path,
    plugin_id: &str,
) -> Result<PathBuf, String> {
    Ok(app_data_dir
        .join("generated-subtitles")
        .join(validate_generated_subtitle_plugin_id(plugin_id)?))
}

pub(in crate::mpv_embed) fn plugin_generated_subtitle_path(
    app_data_dir: &Path,
    plugin_id: &str,
    path: &str,
) -> Result<PathBuf, String> {
    let generated_directory = generated_subtitle_directory(app_data_dir, plugin_id)?;
    let generated_directory = generated_directory
        .canonicalize()
        .map_err(|_| "generated subtitle is not owned by the current plugin".to_string())?;
    let candidate = PathBuf::from(path.trim())
        .canonicalize()
        .map_err(|_| "generated subtitle is not owned by the current plugin".to_string())?;

    if !candidate.starts_with(generated_directory) {
        return Err("generated subtitle is not owned by the current plugin".to_string());
    }
    if !is_supported_subtitle_path(&candidate) {
        return Err(format!(
            "unsupported generated subtitle file: {}",
            candidate.display()
        ));
    }
    Ok(candidate)
}

fn normalize_generated_subtitle_format(format: &str) -> Result<&'static str, String> {
    let normalized = format.trim().to_ascii_lowercase();
    GENERATED_SUBTITLE_EXTENSIONS
        .iter()
        .copied()
        .find(|extension| normalized == *extension)
        .ok_or_else(|| format!("unsupported generated subtitle format: {format}"))
}

fn validate_generated_subtitle_plugin_id(plugin_id: &str) -> Result<&str, String> {
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
        return Err("invalid generated subtitle plugin id".to_string());
    }
    Ok(plugin_id)
}

fn sanitize_generated_subtitle_name(name: &str) -> String {
    let mut sanitized = String::new();
    let mut last_was_separator = false;
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator && !sanitized.is_empty() {
            sanitized.push('-');
            last_was_separator = true;
        }
        if sanitized.len() >= 64 {
            break;
        }
    }
    while sanitized.ends_with('-') {
        sanitized.pop();
    }
    if sanitized.is_empty() {
        "generated-subtitle".to_string()
    } else {
        sanitized
    }
}

fn is_matching_sidecar_stem(path: &Path, media_stem: &str) -> bool {
    let Some(candidate_stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    if candidate_stem == media_stem {
        return true;
    }

    candidate_stem
        .strip_prefix(media_stem)
        .and_then(|suffix| suffix.chars().next())
        .is_some_and(|separator| matches!(separator, '.' | '-' | '_'))
}

fn sidecar_sort_key(path: &Path, media_stem: &str) -> (u8, String) {
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let exact_rank = if file_stem == media_stem { 0 } else { 1 };
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    (exact_rank, file_name)
}

pub(in crate::mpv_embed) fn load_sidecar_subtitles(mpv: &libmpv2::Mpv, media_path: &Path) {
    for (index, subtitle) in discover_sidecar_subtitles(media_path).iter().enumerate() {
        let subtitle_text = subtitle.to_string_lossy();
        let mode = if index == 0 { "select" } else { "auto" };
        let _ = mpv.command("sub-add", &[subtitle_text.as_ref(), mode]);
    }
}
