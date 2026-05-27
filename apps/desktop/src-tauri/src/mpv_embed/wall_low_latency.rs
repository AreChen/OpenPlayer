use super::{MpvWallLatencyMode, MpvWallPlaybackOptions, MpvWallRtspTransport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::mpv_embed) enum WallLowLatencyTuning {
    SetProperty(String, String),
}

pub(in crate::mpv_embed) fn wall_low_latency_tuning_for_url(
    url: &str,
    options: &MpvWallPlaybackOptions,
) -> Option<Vec<WallLowLatencyTuning>> {
    let (scheme, rest) = url.trim().split_once("://")?;
    if rest.trim().is_empty() || !matches!(scheme.to_ascii_lowercase().as_str(), "rtsp" | "rtsps") {
        return None;
    }

    let mode = options.latency_mode?;
    if mode == MpvWallLatencyMode::Off {
        return None;
    }

    let transport = options.rtsp_transport.unwrap_or(match mode {
        MpvWallLatencyMode::Aggressive => MpvWallRtspTransport::Udp,
        MpvWallLatencyMode::Off | MpvWallLatencyMode::Stable | MpvWallLatencyMode::Balanced => {
            MpvWallRtspTransport::Tcp
        }
    });
    let buffer_ms = options
        .buffer_ms
        .unwrap_or(default_wall_buffer_ms(mode))
        .clamp(50, 2_000);
    let max_bytes = match mode {
        MpvWallLatencyMode::Off => return None,
        MpvWallLatencyMode::Stable => 4_194_304,
        MpvWallLatencyMode::Balanced => 4_194_304,
        MpvWallLatencyMode::Aggressive => 2_097_152,
    };

    let mut tuning = Vec::new();
    tuning.extend([
        WallLowLatencyTuning::SetProperty("cache".to_string(), "no".to_string()),
        WallLowLatencyTuning::SetProperty("cache-pause".to_string(), "no".to_string()),
        WallLowLatencyTuning::SetProperty(
            "demuxer-readahead-secs".to_string(),
            format_wall_buffer_seconds(buffer_ms),
        ),
        WallLowLatencyTuning::SetProperty("demuxer-max-bytes".to_string(), max_bytes.to_string()),
        WallLowLatencyTuning::SetProperty("demuxer-max-back-bytes".to_string(), "0".to_string()),
        WallLowLatencyTuning::SetProperty("demuxer-lavf-o".to_string(), lavf_options(transport)),
        WallLowLatencyTuning::SetProperty("vd-lavc-threads".to_string(), "1".to_string()),
        WallLowLatencyTuning::SetProperty("video-sync".to_string(), "display-resample".to_string()),
    ]);
    Some(tuning)
}

pub(in crate::mpv_embed) fn configure_wall_low_latency(
    mpv: &libmpv2::Mpv,
    url: &str,
    options: &MpvWallPlaybackOptions,
) {
    let Some(tuning) = wall_low_latency_tuning_for_url(url, options) else {
        return;
    };
    for action in tuning {
        match action {
            WallLowLatencyTuning::SetProperty(name, value) => {
                let _ = mpv.set_property(name.as_str(), value.as_str());
            }
        }
    }
}

fn default_wall_buffer_ms(mode: MpvWallLatencyMode) -> u32 {
    match mode {
        MpvWallLatencyMode::Off => 0,
        MpvWallLatencyMode::Stable => 600,
        MpvWallLatencyMode::Balanced => 500,
        MpvWallLatencyMode::Aggressive => 300,
    }
}

fn lavf_options(transport: MpvWallRtspTransport) -> String {
    let transport_option = match transport {
        MpvWallRtspTransport::Tcp => "rtsp_transport=tcp",
        MpvWallRtspTransport::Udp => "rtsp_transport=udp",
    };
    ["flags=low_delay", transport_option].join(",")
}

fn format_wall_buffer_seconds(buffer_ms: u32) -> String {
    let mut value = format!("{:.3}", f64::from(buffer_ms) / 1_000.0);
    while value.contains('.') && value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    value
}
