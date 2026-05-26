use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileLayout {
    pub(crate) id: String,
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedMpvWallTileRequest {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) rect: MpvWallTileRect,
    pub(crate) muted: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedMpvWallTileLayout {
    pub(crate) id: String,
    pub(crate) rect: MpvWallTileRect,
}

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
    pub(crate) message: Option<String>,
}
