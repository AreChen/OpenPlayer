use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvAudioClipArtifact {
    pub(crate) path: String,
    pub(crate) format: String,
    pub(crate) mime_type: String,
    pub(crate) start: f64,
    pub(crate) duration: f64,
    pub(crate) sample_rate: u32,
    pub(crate) channels: String,
    pub(crate) size_bytes: u64,
    pub(crate) body_base64: Option<String>,
}
