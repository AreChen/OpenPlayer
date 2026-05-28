use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvEmbedSnapshot {
    pub(crate) path: String,
    pub(crate) hwnd: i64,
    pub(crate) status: String,
    pub(crate) ended: bool,
    pub(crate) paused: bool,
    pub(crate) position: f64,
    pub(crate) duration: f64,
    pub(crate) fps: f64,
    pub(crate) speed: f64,
    pub(crate) hwdec: String,
    pub(crate) video_fill: bool,
    pub(crate) subtitle_delay: f64,
    pub(crate) volume: f64,
    pub(crate) tracks: Vec<MpvEmbedTrack>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvEmbedTrack {
    pub(crate) id: i64,
    pub(crate) kind: String,
    pub(crate) title: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) codec: Option<String>,
    pub(crate) selected: bool,
    pub(crate) external: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSubtitleLoadResult {
    pub(crate) path: String,
    pub(crate) snapshot: MpvEmbedSnapshot,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSubtitleTrack {
    pub(crate) id: i64,
    pub(crate) title: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) codec: Option<String>,
    pub(crate) selected: bool,
    pub(crate) path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentSubtitleCue {
    pub(crate) track_id: i64,
    pub(crate) title: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) start: Option<f64>,
    pub(crate) end: Option<f64>,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedSubtitleCue {
    pub(crate) start: f64,
    pub(crate) end: f64,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MpvLoadOptions {
    #[serde(flatten)]
    pub(crate) options: BTreeMap<String, String>,
}
