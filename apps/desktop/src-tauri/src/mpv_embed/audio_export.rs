use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

const DEFAULT_AUDIO_CLIP_DURATION_SECONDS: f64 = 5.0;
const MIN_AUDIO_CLIP_DURATION_SECONDS: f64 = 0.25;
const MAX_AUDIO_CLIP_DURATION_SECONDS: f64 = 30.0;
const DEFAULT_AUDIO_CLIP_SAMPLE_RATE: u32 = 16_000;
const SUPPORTED_AUDIO_CLIP_SAMPLE_RATES: &[u32] = &[16_000, 24_000, 48_000];
const MAX_AUDIO_CLIP_BASE64_BYTES: u64 = 192 * 1024;
const MIN_WAV_OUTPUT_BYTES: u64 = 44;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::mpv_embed) enum AudioClipChannels {
    Mono,
    Stereo,
}

impl AudioClipChannels {
    fn as_mpv_option(&self) -> &'static str {
        match self {
            AudioClipChannels::Mono => "mono",
            AudioClipChannels::Stereo => "stereo",
        }
    }
}

impl std::fmt::Display for AudioClipChannels {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_mpv_option())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::mpv_embed) struct AudioClipExtractRequest {
    pub(crate) start: f64,
    pub(crate) duration: f64,
    pub(crate) sample_rate: u32,
    pub(crate) channels: AudioClipChannels,
    pub(crate) include_base64: bool,
    pub(crate) format: &'static str,
}

pub(in crate::mpv_embed) fn normalize_audio_clip_extract_request(
    start: Option<f64>,
    duration: Option<f64>,
    sample_rate: Option<u32>,
    channels: Option<String>,
    include_base64: bool,
) -> Result<AudioClipExtractRequest, String> {
    let start = round_audio_clip_time(start.unwrap_or(0.0));
    if !start.is_finite() || start < 0.0 {
        return Err("audio clip start must be a finite non-negative number".to_string());
    }

    let duration = round_audio_clip_time(duration.unwrap_or(DEFAULT_AUDIO_CLIP_DURATION_SECONDS));
    if !duration.is_finite()
        || !(MIN_AUDIO_CLIP_DURATION_SECONDS..=MAX_AUDIO_CLIP_DURATION_SECONDS).contains(&duration)
    {
        return Err(format!(
            "audio clip duration must be between {MIN_AUDIO_CLIP_DURATION_SECONDS} and {MAX_AUDIO_CLIP_DURATION_SECONDS} seconds"
        ));
    }

    let sample_rate = sample_rate.unwrap_or(DEFAULT_AUDIO_CLIP_SAMPLE_RATE);
    if !SUPPORTED_AUDIO_CLIP_SAMPLE_RATES.contains(&sample_rate) {
        return Err("audio clip sampleRate must be 16000, 24000, or 48000".to_string());
    }

    let channels = match channels
        .as_deref()
        .unwrap_or("mono")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "mono" => AudioClipChannels::Mono,
        "stereo" => AudioClipChannels::Stereo,
        _ => return Err("audio clip channels must be mono or stereo".to_string()),
    };

    Ok(AudioClipExtractRequest {
        start,
        duration,
        sample_rate,
        channels,
        include_base64,
        format: "wav",
    })
}

pub(in crate::mpv_embed) fn audio_clip_output_path(
    app_data_dir: &Path,
    plugin_id: &str,
    media_path: &str,
    timestamp_ms: u64,
) -> Result<PathBuf, String> {
    let plugin_directory = validate_audio_clip_plugin_id(plugin_id)?;
    let stem = capture_file_stem(media_path);
    Ok(app_data_dir
        .join("audio-clips")
        .join(plugin_directory)
        .join(format!("openplayer-{stem}-{timestamp_ms}.wav")))
}

pub(in crate::mpv_embed) fn export_audio_clip_to_wav(
    media_path: &str,
    output_path: &Path,
    request: &AudioClipExtractRequest,
) -> Result<MpvAudioClipArtifact, String> {
    let parent = output_path
        .parent()
        .ok_or_else(|| "audio clip output path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create audio clip directory: {error}"))?;

    let mpv = create_audio_clip_export_player(output_path, request)?;
    mpv.command("loadfile", &[media_path, "replace"])
        .map_err(|error| format!("mpv audio clip load failed: {error}"))?;
    let timeout = Duration::from_secs_f64((request.duration + 15.0).min(60.0));
    wait_for_audio_clip_export(&mpv, output_path, timeout)?;
    audio_clip_artifact(output_path, request)
}

fn create_audio_clip_export_player(
    output_path: &Path,
    request: &AudioClipExtractRequest,
) -> Result<libmpv2::Mpv, String> {
    prepare_libmpv_numeric_locale()?;
    let output_text = output_path.to_string_lossy().to_string();
    let start = format_audio_clip_time(request.start);
    let duration = format_audio_clip_time(request.duration);
    let sample_rate = request.sample_rate.to_string();
    let channels = request.channels.as_mpv_option();

    libmpv2::Mpv::with_initializer(|initializer| {
        initializer.set_option("vo", "null")?;
        initializer.set_option("vid", "no")?;
        initializer.set_option("ao", "pcm")?;
        initializer.set_option("ao-pcm-file", output_text.as_str())?;
        initializer.set_option("ao-pcm-waveheader", true)?;
        initializer.set_option("audio-channels", channels)?;
        initializer.set_option("audio-samplerate", sample_rate.as_str())?;
        initializer.set_option("start", start.as_str())?;
        initializer.set_option("length", duration.as_str())?;
        initializer.set_option("keep-open", false)?;
        initializer.set_option("load-scripts", false)?;
        initializer.set_option("input-default-bindings", false)?;
        initializer.set_option("input-vo-keyboard", false)?;
        initializer.set_option("osc", false)?;
        Ok(())
    })
    .map_err(|error| format!("mpv audio clip exporter init failed: {error}"))
}

fn wait_for_audio_clip_export(
    mpv: &libmpv2::Mpv,
    output_path: &Path,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(event) = mpv.wait_event(0.05) {
            match event {
                Ok(Event::EndFile(_)) => return ensure_audio_clip_output_has_content(output_path),
                Err(error) => return Err(format!("mpv audio clip extraction failed: {error}")),
                _ => {}
            }
        }
        if Instant::now() >= deadline {
            let _ = mpv.command("stop", &[]);
            return Err("audio clip extraction timed out".to_string());
        }
    }
}

fn audio_clip_artifact(
    output_path: &Path,
    request: &AudioClipExtractRequest,
) -> Result<MpvAudioClipArtifact, String> {
    let metadata = fs::metadata(output_path)
        .map_err(|error| format!("audio clip output was not created: {error}"))?;
    let size_bytes = metadata.len();
    let body_base64 = if request.include_base64 {
        if size_bytes > MAX_AUDIO_CLIP_BASE64_BYTES {
            return Err(format!(
                "audio clip is too large to return as base64: {size_bytes} bytes exceeds {MAX_AUDIO_CLIP_BASE64_BYTES}"
            ));
        }
        Some(
            BASE64_STANDARD.encode(
                fs::read(output_path)
                    .map_err(|error| format!("failed to read audio clip output: {error}"))?,
            ),
        )
    } else {
        None
    };

    Ok(MpvAudioClipArtifact {
        path: output_path.to_string_lossy().to_string(),
        format: request.format.to_string(),
        mime_type: "audio/wav".to_string(),
        start: request.start,
        duration: request.duration,
        sample_rate: request.sample_rate,
        channels: request.channels.to_string(),
        size_bytes,
        body_base64,
    })
}

fn ensure_audio_clip_output_has_content(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("audio clip output was not created: {error}"))?;
    if metadata.len() <= MIN_WAV_OUTPUT_BYTES {
        let _ = fs::remove_file(path);
        return Err("mpv produced an empty audio clip".to_string());
    }
    Ok(())
}

fn validate_audio_clip_plugin_id(plugin_id: &str) -> Result<&str, String> {
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
        return Err("invalid audio clip plugin id".to_string());
    }
    Ok(plugin_id)
}

fn round_audio_clip_time(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn format_audio_clip_time(value: f64) -> String {
    format!("{value:.3}")
}
