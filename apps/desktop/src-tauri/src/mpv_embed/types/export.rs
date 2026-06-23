use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvMediaSegmentExportArtifact {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) format: String,
    pub(crate) mime_type: String,
    pub(crate) start: f64,
    pub(crate) duration: f64,
    pub(crate) size_bytes: u64,
}
