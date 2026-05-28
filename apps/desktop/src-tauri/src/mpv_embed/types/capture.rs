use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MpvRecordingSession {
    pub(crate) path: String,
    pub(crate) format: String,
    pub(crate) method: MpvRecordingMethod,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MpvRecordingMethod {
    StreamRecord,
    DumpCache { start_position: f64 },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvCaptureArtifact {
    pub(crate) path: String,
    pub(crate) copied_to_clipboard: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvFrameCaptureArtifact {
    pub(crate) path: String,
    pub(crate) format: String,
    pub(crate) mime_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) body_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvRecordingState {
    pub(crate) active: bool,
    pub(crate) path: Option<String>,
    pub(crate) format: Option<String>,
}
