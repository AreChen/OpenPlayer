use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeModule {
    pub id: String,
    pub protocol: String,
    pub methods: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_adapter: Option<String>,
    pub targets: BTreeMap<String, NativeTarget>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeTarget {
    pub entry: String,
    pub sha256: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeModuleInfo {
    pub id: String,
    pub protocol: String,
    pub methods: Vec<String>,
    pub supported: bool,
    pub running: bool,
}

#[derive(Clone)]
pub(crate) struct ModuleLaunch {
    pub plugin_id: String,
    pub plugin_name: String,
    pub plugin_version: String,
    pub language_mode: String,
    pub executable: PathBuf,
    #[cfg_attr(not(all(windows, feature = "mpv-embed")), allow(dead_code))]
    pub package_root: PathBuf,
    pub args: Vec<String>,
    pub module: NativeModule,
}
