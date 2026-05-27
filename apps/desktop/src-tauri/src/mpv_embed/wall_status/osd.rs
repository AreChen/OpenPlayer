#[cfg(windows)]
pub(in crate::mpv_embed) fn configure_wall_osd(mpv: &libmpv2::Mpv) {
    let _ = mpv.set_property("osd-align-x", "left");
    let _ = mpv.set_property("osd-align-y", "top");
    let _ = mpv.set_property("osd-margin-x", 12);
    let _ = mpv.set_property("osd-margin-y", 12);
    let _ = mpv.set_property("osd-font-size", 18);
    let _ = mpv.set_property("osd-bold", true);
    let _ = mpv.set_property("osd-color", "#f1c66b");
    let _ = mpv.set_property("osd-border-color", "#120f08");
    let _ = mpv.set_property("osd-border-size", 1.8);
    let _ = mpv.set_property("osd-shadow-color", "#000000");
    let _ = mpv.set_property("osd-shadow-offset", 1.0);
    let _ = mpv.set_property("osd-back-color", "#99000000");
}

#[cfg(any(windows, test))]
pub(in crate::mpv_embed) fn format_wall_buffer_millis(buffer_seconds: Option<f64>) -> String {
    buffer_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| format!("{} ms", (value * 1000.0).round() as i64))
        .unwrap_or_else(|| "-- ms".to_string())
}

#[cfg(any(windows, test))]
pub(in crate::mpv_embed) fn format_wall_bitrate(bits_per_second: Option<f64>) -> String {
    let Some(bits_per_second) = bits_per_second.filter(|value| value.is_finite() && *value > 0.0)
    else {
        return "--".to_string();
    };
    if bits_per_second >= 1_000_000.0 {
        format!("{:.1} Mbps", bits_per_second / 1_000_000.0)
    } else {
        format!("{} Kbps", (bits_per_second / 1000.0).round() as i64)
    }
}

#[cfg(any(windows, test))]
pub(in crate::mpv_embed) fn format_wall_transport_latency(
    latency_ms: Option<f64>,
    source: Option<&str>,
) -> Option<String> {
    let latency_ms = latency_ms.filter(|value| value.is_finite() && *value >= 0.0)?;
    match source {
        Some(crate::mpv_embed::RTSP_RECEIVE_LATENCY_SOURCE) => {
            Some(format!("RTCP {} ms", latency_ms.round() as i64))
        }
        _ => None,
    }
}

#[cfg(windows)]
pub(in crate::mpv_embed) fn update_wall_osd(
    mpv: &libmpv2::Mpv,
    buffer_seconds: Option<f64>,
    bitrate_bps: Option<f64>,
    transport_latency_ms: Option<f64>,
    transport_latency_source: Option<&str>,
) {
    let mut parts = Vec::new();
    if let Some(latency) =
        format_wall_transport_latency(transport_latency_ms, transport_latency_source)
    {
        parts.push(latency);
    }
    parts.push(format!("BUF {}", format_wall_buffer_millis(buffer_seconds)));
    parts.push(format_wall_bitrate(bitrate_bps));
    let text = parts.join(" · ");
    let _ = mpv.command("show-text", &[text.as_str(), "1500", "1"]);
}
