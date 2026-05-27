mod rtcp;
mod rtp;
mod rtsp;
#[cfg(windows)]
mod session;
mod state;

pub(in crate::mpv_embed) const RTSP_RECEIVE_LATENCY_SOURCE: &str = "rtcp-rtp-receive";

pub(in crate::mpv_embed) use rtcp::RtcpSenderReport;
#[cfg(test)]
pub(in crate::mpv_embed) use rtcp::parse_rtcp_sender_report;
#[cfg(test)]
pub(in crate::mpv_embed) use rtp::parse_rtp_header;
#[cfg(test)]
pub(in crate::mpv_embed) use rtsp::rtsp_rtp_info_timestamp;
#[cfg(windows)]
pub(in crate::mpv_embed) use session::{RtspTelemetryHandle, start_rtsp_receive_telemetry};
#[cfg(windows)]
pub(in crate::mpv_embed) use state::RtspTelemetrySnapshot;
pub(in crate::mpv_embed) use state::RtspTelemetryState;
