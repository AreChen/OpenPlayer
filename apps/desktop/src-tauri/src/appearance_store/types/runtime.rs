use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginRuntime {
    pub(in crate::appearance_store) kind: PluginRuntimeKind,
    pub(in crate::appearance_store) entry: Option<String>,
    pub(in crate::appearance_store) sandbox: Option<String>,
    #[serde(default)]
    pub(in crate::appearance_store) events: Vec<String>,
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self {
            kind: PluginRuntimeKind::Manifest,
            entry: None,
            sandbox: None,
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(in crate::appearance_store) enum PluginRuntimeKind {
    #[serde(rename = "manifest")]
    Manifest,
    #[serde(rename = "webviewJs")]
    WebviewJs,
    #[serde(rename = "wasm")]
    Wasm,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeSource {
    pub(in crate::appearance_store) plugin_id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) version: String,
    pub(in crate::appearance_store) entry: String,
    pub(in crate::appearance_store) script: String,
    pub(in crate::appearance_store) permissions: Vec<String>,
    pub(in crate::appearance_store) events: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginViewHtml {
    pub(in crate::appearance_store) plugin_id: String,
    pub(in crate::appearance_store) view_id: String,
    pub(in crate::appearance_store) title: String,
    pub(in crate::appearance_store) html: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginStorageInfo {
    pub(in crate::appearance_store) plugin_id: String,
    pub(in crate::appearance_store) schema_version: u32,
    pub(in crate::appearance_store) manifest_version: u32,
    pub(in crate::appearance_store) keys: Vec<String>,
}
