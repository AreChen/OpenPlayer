use std::collections::HashMap;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginNetworkRequestArgs {
    pub(super) url: String,
    pub(super) method: Option<String>,
    pub(super) headers: Option<HashMap<String, String>>,
    pub(super) body: Option<String>,
    pub(super) body_file: Option<PluginNetworkRequestBodyFile>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) response_type: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginNetworkRequestBodyFile {
    pub(super) path: String,
    pub(super) content_type: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginNetworkResponse {
    pub(super) url: String,
    pub(super) status: u16,
    pub(super) ok: bool,
    pub(super) headers: HashMap<String, String>,
    pub(super) text: String,
    pub(super) body_base64: Option<String>,
}
