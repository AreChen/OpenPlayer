use super::*;

const DEFAULT_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS: f64 = 10.0;
const MIN_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS: f64 = 0.25;
const MAX_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS: f64 = 600.0;
const MIN_MEDIA_SEGMENT_EXPORT_OUTPUT_BYTES: u64 = 512;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::mpv_embed) enum MediaSegmentExportKind {
    Audio,
    Video,
}

impl MediaSegmentExportKind {
    fn as_str(self) -> &'static str {
        match self {
            MediaSegmentExportKind::Audio => "audio",
            MediaSegmentExportKind::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::mpv_embed) struct MediaSegmentExportFormat {
    pub(crate) id: &'static str,
    pub(crate) extension: &'static str,
    pub(crate) mime_type: &'static str,
    container: &'static str,
    audio_codec: &'static str,
    video_codec: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::mpv_embed) struct MediaSegmentExportRequest {
    pub(crate) kind: MediaSegmentExportKind,
    pub(crate) format: MediaSegmentExportFormat,
    pub(crate) start: f64,
    pub(crate) duration: f64,
    pub(crate) file_stem: Option<String>,
}

pub(in crate::mpv_embed) fn normalize_media_segment_export_request(
    kind: Option<String>,
    format: Option<String>,
    start: Option<f64>,
    duration: Option<f64>,
    file_name: Option<String>,
) -> Result<MediaSegmentExportRequest, String> {
    let kind = match kind
        .as_deref()
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .unwrap_or("video")
        .to_ascii_lowercase()
        .as_str()
    {
        "audio" => MediaSegmentExportKind::Audio,
        "video" => MediaSegmentExportKind::Video,
        _ => return Err("media segment export kind must be audio or video".to_string()),
    };

    let format = normalize_media_segment_export_format(kind, format)?;
    let start = round_media_segment_export_time(start.unwrap_or(0.0));
    if !start.is_finite() || start < 0.0 {
        return Err("media segment export start must be a finite non-negative number".to_string());
    }

    let duration = round_media_segment_export_time(
        duration.unwrap_or(DEFAULT_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS),
    );
    if !duration.is_finite()
        || !(MIN_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS..=MAX_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS)
            .contains(&duration)
    {
        return Err(format!(
            "media segment export duration must be between {MIN_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS} and {MAX_MEDIA_SEGMENT_EXPORT_DURATION_SECONDS} seconds"
        ));
    }

    let file_stem = file_name
        .as_deref()
        .map(capture_file_stem)
        .filter(|stem| !stem.is_empty());

    Ok(MediaSegmentExportRequest {
        kind,
        format,
        start,
        duration,
        file_stem,
    })
}

pub(in crate::mpv_embed) fn media_segment_export_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    request: &MediaSegmentExportRequest,
) -> PathBuf {
    let stem = request
        .file_stem
        .clone()
        .unwrap_or_else(|| capture_file_stem(media_path));
    directory.join(format!(
        "openplayer-{stem}-{timestamp_ms}.{}",
        request.format.extension
    ))
}

pub(in crate::mpv_embed) fn media_segment_export_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create media segment export directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().download_dir() {
        directory.push("OpenPlayer");
        directory.push("Exports");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve media segment export directory: {error}"))?;
    directory.push("exports");
    Ok(directory)
}

pub(in crate::mpv_embed) fn export_media_segment_to_file(
    media_path: &str,
    output_path: &Path,
    request: &MediaSegmentExportRequest,
) -> Result<MpvMediaSegmentExportArtifact, String> {
    let parent = output_path
        .parent()
        .ok_or_else(|| "media segment export output path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create media segment export directory: {error}"))?;

    let mpv = create_media_segment_export_player(output_path, request)?;
    mpv.command("loadfile", &[media_path, "replace"])
        .map_err(|error| format!("mpv media segment export load failed: {error}"))?;
    let timeout = Duration::from_secs_f64((request.duration + 30.0).min(900.0));
    wait_for_media_segment_export(&mpv, output_path, timeout)?;
    media_segment_export_artifact(output_path, request)
}

fn normalize_media_segment_export_format(
    kind: MediaSegmentExportKind,
    format: Option<String>,
) -> Result<MediaSegmentExportFormat, String> {
    let default_format = match kind {
        MediaSegmentExportKind::Audio => "mp3",
        MediaSegmentExportKind::Video => "mp4",
    };
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or(default_format)
        .to_ascii_lowercase();
    match (kind, format.as_str()) {
        (MediaSegmentExportKind::Audio, "wav") => Ok(MediaSegmentExportFormat {
            id: "wav",
            extension: "wav",
            mime_type: "audio/wav",
            container: "wav",
            audio_codec: "pcm_s16le",
            video_codec: None,
        }),
        (MediaSegmentExportKind::Audio, "mp3") => Ok(MediaSegmentExportFormat {
            id: "mp3",
            extension: "mp3",
            mime_type: "audio/mpeg",
            container: "mp3",
            audio_codec: "libmp3lame",
            video_codec: None,
        }),
        (MediaSegmentExportKind::Audio, "m4a") => Ok(MediaSegmentExportFormat {
            id: "m4a",
            extension: "m4a",
            mime_type: "audio/mp4",
            container: "mp4",
            audio_codec: "aac",
            video_codec: None,
        }),
        (MediaSegmentExportKind::Audio, "flac") => Ok(MediaSegmentExportFormat {
            id: "flac",
            extension: "flac",
            mime_type: "audio/flac",
            container: "flac",
            audio_codec: "flac",
            video_codec: None,
        }),
        (MediaSegmentExportKind::Video, "mp4") => Ok(MediaSegmentExportFormat {
            id: "mp4",
            extension: "mp4",
            mime_type: "video/mp4",
            container: "mp4",
            audio_codec: "aac",
            video_codec: Some("mpeg4"),
        }),
        (MediaSegmentExportKind::Video, "mkv") => Ok(MediaSegmentExportFormat {
            id: "mkv",
            extension: "mkv",
            mime_type: "video/x-matroska",
            container: "matroska",
            audio_codec: "aac",
            video_codec: Some("mpeg4"),
        }),
        (MediaSegmentExportKind::Audio, _) => {
            Err(format!("unsupported audio segment export format: {format}"))
        }
        (MediaSegmentExportKind::Video, _) => {
            Err(format!("unsupported video segment export format: {format}"))
        }
    }
}

fn create_media_segment_export_player(
    output_path: &Path,
    request: &MediaSegmentExportRequest,
) -> Result<libmpv2::Mpv, String> {
    prepare_libmpv_numeric_locale()?;
    let output_text = output_path.to_string_lossy().to_string();
    let start = format_media_segment_export_time(request.start);
    let duration = format_media_segment_export_time(request.duration);

    libmpv2::Mpv::with_initializer(|initializer| {
        initializer.set_option("o", output_text.as_str())?;
        initializer.set_option("of", request.format.container)?;
        initializer.set_option("oac", request.format.audio_codec)?;
        if let Some(video_codec) = request.format.video_codec {
            initializer.set_option("ovc", video_codec)?;
        } else {
            initializer.set_option("vid", "no")?;
        }
        initializer.set_option("start", start.as_str())?;
        initializer.set_option("length", duration.as_str())?;
        initializer.set_option("keep-open", false)?;
        initializer.set_option("load-scripts", false)?;
        initializer.set_option("input-default-bindings", false)?;
        initializer.set_option("input-vo-keyboard", false)?;
        initializer.set_option("osc", false)?;
        Ok(())
    })
    .map_err(|error| format!("mpv media segment exporter init failed: {error}"))
}

fn wait_for_media_segment_export(
    mpv: &libmpv2::Mpv,
    output_path: &Path,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let mut end_file_seen = false;
    loop {
        if let Some(event) = mpv.wait_event(0.05) {
            match event {
                Ok(Event::EndFile(_)) => {
                    end_file_seen = true;
                    let _ = mpv.command("quit", &[]);
                }
                Ok(Event::Shutdown) if end_file_seen => {
                    return ensure_media_segment_export_output(output_path);
                }
                Ok(Event::Shutdown) => {
                    return Err("mpv media segment export stopped before finishing".to_string());
                }
                Err(error) => return Err(format!("mpv media segment export failed: {error}")),
                _ => {}
            }
        }
        if Instant::now() >= deadline {
            let _ = mpv.command("stop", &[]);
            return Err("media segment export timed out".to_string());
        }
    }
}

pub(in crate::mpv_embed) fn ensure_media_segment_export_output(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("media segment export was not created: {error}"))?;
    if metadata.len() <= MIN_MEDIA_SEGMENT_EXPORT_OUTPUT_BYTES {
        let _ = fs::remove_file(path);
        return Err("mpv produced an empty media segment export".to_string());
    }
    Ok(())
}

fn media_segment_export_artifact(
    output_path: &Path,
    request: &MediaSegmentExportRequest,
) -> Result<MpvMediaSegmentExportArtifact, String> {
    let metadata = fs::metadata(output_path)
        .map_err(|error| format!("media segment export was not created: {error}"))?;
    Ok(MpvMediaSegmentExportArtifact {
        path: output_path.to_string_lossy().to_string(),
        kind: request.kind.as_str().to_string(),
        format: request.format.id.to_string(),
        mime_type: request.format.mime_type.to_string(),
        start: request.start,
        duration: request.duration,
        size_bytes: metadata.len(),
    })
}

fn round_media_segment_export_time(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn format_media_segment_export_time(value: f64) -> String {
    format!("{value:.3}")
}
