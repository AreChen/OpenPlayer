use serde::{Deserialize, Serialize};

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileRequest {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) muted: Option<bool>,
    #[serde(default)]
    pub(crate) playback: Option<MpvWallPlaybackOptions>,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MpvWallLatencyMode {
    Off,
    Stable,
    Balanced,
    Aggressive,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum MpvWallRtspTransport {
    Tcp,
    Udp,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MpvWallPlaybackOptions {
    #[serde(default)]
    pub(crate) latency_mode: Option<MpvWallLatencyMode>,
    #[serde(default)]
    pub(crate) rtsp_transport: Option<MpvWallRtspTransport>,
    #[serde(default)]
    pub(crate) buffer_ms: Option<u32>,
}

#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileLayout {
    pub(crate) id: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[cfg(any(windows, test))]
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) struct NormalizedMpvWallTileRequest {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) rect: MpvWallTileRect,
    pub(crate) muted: bool,
    pub(crate) playback: MpvWallPlaybackOptions,
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone)]
pub(crate) struct NormalizedMpvWallTileLayout {
    pub(crate) id: String,
    pub(crate) rect: MpvWallTileRect,
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MpvWallTileRect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileSnapshot {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) status: String,
    pub(crate) latency_seconds: Option<f64>,
    pub(crate) buffer_seconds: Option<f64>,
    pub(crate) bitrate_bps: Option<f64>,
    pub(crate) transport_latency_ms: Option<f64>,
    pub(crate) transport_latency_source: Option<String>,
    pub(crate) message: Option<String>,
}
