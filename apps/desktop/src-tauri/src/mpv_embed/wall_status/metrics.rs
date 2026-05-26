#[cfg(any(windows, test))]
pub(in crate::mpv_embed) fn combine_wall_bitrate(
    video_bitrate: Option<f64>,
    audio_bitrate: Option<f64>,
    raw_input_bytes_per_second: Option<f64>,
) -> Option<f64> {
    let track_bitrate = video_bitrate
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(0.0)
        + audio_bitrate
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(0.0);
    if track_bitrate > 0.0 {
        return Some(track_bitrate);
    }

    raw_input_bytes_per_second
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|bytes_per_second| bytes_per_second * 8.0)
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn read_finite_mpv_property(
    mpv: &libmpv2::Mpv,
    property: &str,
) -> Option<f64> {
    mpv.get_property::<f64>(property)
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .or_else(|| {
            mpv.get_property::<i64>(property)
                .ok()
                .map(|value| value as f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
        })
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn read_wall_buffer(mpv: &libmpv2::Mpv) -> Option<f64> {
    read_finite_mpv_property(mpv, "demuxer-cache-duration")
        .or_else(|| read_finite_mpv_property(mpv, "demuxer-cache-state/cache-duration"))
        .or_else(|| read_finite_mpv_property(mpv, "cache-duration"))
        .or_else(|| {
            let cache_time = read_finite_mpv_property(mpv, "demuxer-cache-time")?;
            let position = read_finite_mpv_property(mpv, "time-pos")?;
            let buffered = cache_time - position;
            (buffered.is_finite() && buffered >= 0.0).then_some(buffered)
        })
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn read_wall_bitrate(mpv: &libmpv2::Mpv) -> Option<f64> {
    combine_wall_bitrate(
        read_finite_mpv_property(mpv, "video-bitrate"),
        read_finite_mpv_property(mpv, "audio-bitrate"),
        read_finite_mpv_property(mpv, "cache-speed")
            .or_else(|| read_finite_mpv_property(mpv, "demuxer-cache-state/raw-input-rate")),
    )
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn read_wall_bool_property(mpv: &libmpv2::Mpv, property: &str) -> bool {
    mpv.get_property::<bool>(property).unwrap_or(false)
}
