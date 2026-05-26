use std::collections::HashMap;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginNetworkRequestArgs {
    pub(super) url: String,
    pub(super) method: Option<String>,
    pub(super) headers: Option<HashMap<String, String>>,
    pub(super) body: Option<String>,
    pub(super) timeout_ms: Option<u64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginNetworkResponse {
    pub(super) url: String,
    pub(super) status: u16,
    pub(super) ok: bool,
    pub(super) headers: HashMap<String, String>,
    pub(super) text: String,
}
