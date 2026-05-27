use super::{RTSP_RECEIVE_LATENCY_SOURCE, RtcpSenderReport};
use crate::mpv_embed::{rtcp_ntp_to_unix_ns, rtp_sender_time_ns, transport_latency_ms};

#[derive(Debug, Clone)]
pub(in crate::mpv_embed) struct RtspTelemetrySnapshot {
    pub(in crate::mpv_embed) latency_ms: f64,
    pub(in crate::mpv_embed) source: &'static str,
}

#[derive(Debug, Default)]
pub(in crate::mpv_embed) struct RtspTelemetryState {
    latest_rtp_timestamp: Option<u32>,
    latest_rtp_receive_time_ns: Option<i128>,
    latest_sender_report: Option<RtcpSenderReport>,
}

impl RtspTelemetryState {
    pub(in crate::mpv_embed) fn update_rtp(&mut self, rtp_timestamp: u32, receive_time_ns: i128) {
        self.latest_rtp_timestamp = Some(rtp_timestamp);
        self.latest_rtp_receive_time_ns = Some(receive_time_ns);
    }

    pub(in crate::mpv_embed) fn update_sender_report(&mut self, report: RtcpSenderReport) {
        self.latest_sender_report = Some(report);
    }

    pub(in crate::mpv_embed) fn snapshot(&self) -> Option<RtspTelemetrySnapshot> {
        let rtp_timestamp = self.latest_rtp_timestamp?;
        let receive_time_ns = self.latest_rtp_receive_time_ns?;
        let report = self.latest_sender_report?;
        let sender_report_ntp_ns = rtcp_ntp_to_unix_ns(report.ntp_seconds, report.ntp_fraction)?;
        let sender_time_ns = rtp_sender_time_ns(
            rtp_timestamp,
            report.rtp_timestamp,
            sender_report_ntp_ns,
            90_000,
        )?;
        let latency_ms = transport_latency_ms(receive_time_ns, sender_time_ns)?;
        (0.0..60_000.0)
            .contains(&latency_ms)
            .then_some(RtspTelemetrySnapshot {
                latency_ms,
                source: RTSP_RECEIVE_LATENCY_SOURCE,
            })
    }
}
