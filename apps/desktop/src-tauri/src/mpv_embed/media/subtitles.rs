use super::*;

pub(in crate::mpv_embed) const MAX_GENERATED_SUBTITLE_BYTES: usize = 2 * 1024 * 1024;
const MAX_GENERATED_SUBTITLE_CUES: usize = 10_000;
const GENERATED_SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub"];
const GENERATED_SUBTITLE_CUE_FORMATS: &[&str] = &["srt", "vtt"];

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

pub(in crate::mpv_embed) fn format_generated_subtitle_cues(
    format: &str,
    cues: &[GeneratedSubtitleCue],
) -> Result<String, String> {
    let format = normalize_generated_subtitle_cue_format(format)?;
    if cues.is_empty() {
        return Err("generated subtitle cues cannot be empty".to_string());
    }
    if cues.len() > MAX_GENERATED_SUBTITLE_CUES {
        return Err(format!(
            "generated subtitle cues exceed maximum count: {} > {}",
            cues.len(),
            MAX_GENERATED_SUBTITLE_CUES
        ));
    }

    let mut cues = cues.to_vec();
    cues.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
    });

    match format {
        "srt" => format_generated_srt_cues(&cues),
        "vtt" => format_generated_vtt_cues(&cues),
        _ => Err(format!(
            "unsupported generated subtitle cue format: {format}"
        )),
    }
}

pub(in crate::mpv_embed) fn append_generated_subtitle_cues_file(
    path: &Path,
    cues: &[GeneratedSubtitleCue],
) -> Result<(), String> {
    let format = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| "generated subtitle file has no format extension".to_string())?;
    let format = normalize_generated_subtitle_cue_format(format)?;
    let existing = fs::read_to_string(path)
        .map_err(|error| format!("failed to read generated subtitle file: {error}"))?;
    let append_content = match format {
        "srt" => format_generated_srt_cues_from_index(cues, next_srt_cue_index(&existing))?,
        "vtt" => format_generated_vtt_cues_for_append(cues)?,
        _ => {
            return Err(format!(
                "unsupported generated subtitle cue format: {format}"
            ));
        }
    };
    let mut combined = existing.trim_end().to_string();
    if !combined.is_empty() {
        combined.push_str("\n\n");
    }
    combined.push_str(append_content.trim_start());
    combined.push('\n');

    let byte_len = combined.len();
    if byte_len > MAX_GENERATED_SUBTITLE_BYTES {
        return Err(format!(
            "generated subtitle content is too large: {byte_len} bytes exceeds {MAX_GENERATED_SUBTITLE_BYTES}"
        ));
    }
    fs::write(path, combined.as_bytes())
        .map_err(|error| format!("failed to append generated subtitle cues: {error}"))
}

pub(in crate::mpv_embed) fn read_generated_subtitle_file(
    path: &Path,
) -> Result<GeneratedSubtitleContent, String> {
    let format = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| "generated subtitle file has no format extension".to_string())?;
    let format = normalize_generated_subtitle_format(format)?.to_string();
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read generated subtitle file: {error}"))?;
    let byte_len = content.len();
    if byte_len > MAX_GENERATED_SUBTITLE_BYTES {
        return Err(format!(
            "generated subtitle content is too large: {byte_len} bytes exceeds {MAX_GENERATED_SUBTITLE_BYTES}"
        ));
    }
    let cues = parse_generated_subtitle_cues(&format, &content)
        .ok()
        .flatten();

    Ok(GeneratedSubtitleContent {
        format,
        content,
        cues,
    })
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

fn normalize_generated_subtitle_cue_format(format: &str) -> Result<&'static str, String> {
    let normalized = format.trim().to_ascii_lowercase();
    GENERATED_SUBTITLE_CUE_FORMATS
        .iter()
        .copied()
        .find(|extension| normalized == *extension)
        .ok_or_else(|| format!("unsupported generated subtitle cue format: {format}"))
}

fn format_generated_srt_cues(cues: &[GeneratedSubtitleCue]) -> Result<String, String> {
    format_generated_srt_cues_from_index(cues, 1)
}

fn format_generated_srt_cues_from_index(
    cues: &[GeneratedSubtitleCue],
    first_index: usize,
) -> Result<String, String> {
    let mut output = String::new();
    for (index, cue) in cues.iter().enumerate() {
        validate_generated_subtitle_cue(cue)?;
        output.push_str(&(first_index + index).to_string());
        output.push('\n');
        output.push_str(&format!(
            "{} --> {}\n",
            format_generated_subtitle_timecode(cue.start, ','),
            format_generated_subtitle_timecode(cue.end, ',')
        ));
        output.push_str(&normalize_generated_subtitle_cue_text(&cue.text));
        output.push_str("\n\n");
    }
    Ok(output)
}

fn format_generated_vtt_cues(cues: &[GeneratedSubtitleCue]) -> Result<String, String> {
    let mut output = String::from("WEBVTT\n\n");
    for cue in cues {
        validate_generated_subtitle_cue(cue)?;
        output.push_str(&format!(
            "{} --> {}\n",
            format_generated_subtitle_timecode(cue.start, '.'),
            format_generated_subtitle_timecode(cue.end, '.')
        ));
        output.push_str(&normalize_generated_subtitle_cue_text(&cue.text));
        output.push_str("\n\n");
    }
    Ok(output)
}

fn format_generated_vtt_cues_for_append(cues: &[GeneratedSubtitleCue]) -> Result<String, String> {
    let mut output = String::new();
    for cue in cues {
        validate_generated_subtitle_cue(cue)?;
        output.push_str(&format!(
            "{} --> {}\n",
            format_generated_subtitle_timecode(cue.start, '.'),
            format_generated_subtitle_timecode(cue.end, '.')
        ));
        output.push_str(&normalize_generated_subtitle_cue_text(&cue.text));
        output.push_str("\n\n");
    }
    Ok(output)
}

fn validate_generated_subtitle_cue(cue: &GeneratedSubtitleCue) -> Result<(), String> {
    if !cue.start.is_finite() || !cue.end.is_finite() || cue.start < 0.0 || cue.end <= cue.start {
        return Err(
            "generated subtitle cues require finite non-overlapping start/end times".to_string(),
        );
    }
    if cue.text.trim().is_empty() {
        return Err("generated subtitle cues require non-empty text".to_string());
    }
    Ok(())
}

fn next_srt_cue_index(content: &str) -> usize {
    let normalized = content.replace("\r\n", "\n");
    let count = normalized
        .split("\n\n")
        .filter_map(|block| block.lines().find(|line| !line.trim().is_empty()))
        .filter(|line| line.trim().parse::<usize>().is_ok())
        .count();
    count + 1
}

fn parse_generated_subtitle_cues(
    format: &str,
    content: &str,
) -> Result<Option<Vec<GeneratedSubtitleCue>>, String> {
    normalize_generated_subtitle_cue_format(format)?;
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut cues = Vec::new();

    for block in normalized.split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty())
            .collect();
        let Some(time_index) = lines.iter().position(|line| line.contains("-->")) else {
            continue;
        };
        let Some((start, end)) = parse_generated_subtitle_time_range(lines[time_index]) else {
            continue;
        };
        let text = lines
            .iter()
            .skip(time_index + 1)
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if text.is_empty() {
            continue;
        }
        cues.push(GeneratedSubtitleCue { start, end, text });
    }

    Ok(Some(cues))
}

fn parse_generated_subtitle_time_range(line: &str) -> Option<(f64, f64)> {
    let (start, rest) = line.split_once("-->")?;
    let end = rest.split_whitespace().next()?;
    Some((
        parse_generated_subtitle_timecode(start.trim())?,
        parse_generated_subtitle_timecode(end.trim())?,
    ))
}

fn parse_generated_subtitle_timecode(value: &str) -> Option<f64> {
    let normalized = value.replace(',', ".");
    let mut parts = normalized.split(':');
    let hours = parts.next()?.parse::<u64>().ok()?;
    let minutes = parts.next()?.parse::<u64>().ok()?;
    let seconds_part = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let (seconds, millis) = seconds_part.split_once('.').unwrap_or((seconds_part, "0"));
    let seconds = seconds.parse::<u64>().ok()?;
    let millis = format!("{millis:0<3}");
    let millis = millis.get(..3)?.parse::<u64>().ok()?;
    Some((hours * 3600 + minutes * 60 + seconds) as f64 + millis as f64 / 1000.0)
}

fn normalize_generated_subtitle_cue_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_generated_subtitle_timecode(seconds: f64, millisecond_separator: char) -> String {
    let total_ms = (seconds * 1000.0).round().max(0.0) as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms / 60_000) % 60;
    let seconds = (total_ms / 1_000) % 60;
    let milliseconds = total_ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}{millisecond_separator}{milliseconds:03}")
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
