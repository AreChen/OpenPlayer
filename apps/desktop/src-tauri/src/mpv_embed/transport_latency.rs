const NTP_UNIX_EPOCH_OFFSET_SECONDS: u64 = 2_208_988_800;
const NANOS_PER_SECOND: i128 = 1_000_000_000;

pub(in crate::mpv_embed) fn signed_rtp_delta_32(current: u32, reference: u32) -> i64 {
    current.wrapping_sub(reference) as i32 as i64
}

pub(in crate::mpv_embed) fn rtcp_ntp_to_unix_ns(
    ntp_seconds: u32,
    ntp_fraction: u32,
) -> Option<i128> {
    let unix_seconds = u64::from(ntp_seconds).checked_sub(NTP_UNIX_EPOCH_OFFSET_SECONDS)?;
    let fractional_ns = (u128::from(ntp_fraction) * 1_000_000_000u128) >> 32;
    Some(i128::from(unix_seconds) * NANOS_PER_SECOND + fractional_ns as i128)
}

pub(in crate::mpv_embed) fn rtp_sender_time_ns(
    rtp_timestamp: u32,
    sender_report_rtp_timestamp: u32,
    sender_report_ntp_unix_ns: i128,
    rtp_clock_hz: u32,
) -> Option<i128> {
    if rtp_clock_hz == 0 {
        return None;
    }
    let delta = i128::from(signed_rtp_delta_32(
        rtp_timestamp,
        sender_report_rtp_timestamp,
    ));
    Some(sender_report_ntp_unix_ns + (delta * NANOS_PER_SECOND) / i128::from(rtp_clock_hz))
}

pub(in crate::mpv_embed) fn transport_latency_ms(
    display_time_ns: i128,
    sender_time_ns: i128,
) -> Option<f64> {
    let latency_ms = (display_time_ns - sender_time_ns) as f64 / 1_000_000.0;
    latency_ms.is_finite().then_some(latency_ms)
}
