use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{PluginRuntime, ThemeManifest, default_plugin_action_args, default_plugin_api_version};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct StoredPluginInstall {
    pub(in crate::appearance_store) package_kind: String,
    pub(in crate::appearance_store) install_path: String,
    pub(in crate::appearance_store) installed_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginManifest {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) version: String,
    #[serde(default = "default_plugin_api_version")]
    pub(in crate::appearance_store) api_version: String,
    pub(in crate::appearance_store) min_host_version: Option<String>,
    pub(in crate::appearance_store) author: Option<String>,
    pub(in crate::appearance_store) update_url: Option<String>,
    pub(in crate::appearance_store) description: Option<String>,
    pub(in crate::appearance_store) entry: ThemePluginEntry,
    #[serde(default)]
    pub(in crate::appearance_store) runtime: PluginRuntime,
    pub(in crate::appearance_store) contributes: PluginContributions,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginContributions {
    #[serde(default)]
    pub(in crate::appearance_store) themes: Vec<ThemeManifest>,
    #[serde(default)]
    pub(in crate::appearance_store) capabilities: Vec<PluginCapabilityManifest>,
    #[serde(default)]
    pub(in crate::appearance_store) settings: Vec<PluginSettingManifest>,
    #[serde(default)]
    pub(in crate::appearance_store) actions: Vec<PluginActionManifest>,
    #[serde(default)]
    pub(in crate::appearance_store) views: Vec<PluginViewManifest>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(in crate::appearance_store) enum ThemePluginEntry {
    #[serde(rename = "manifest")]
    Manifest,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginCapabilityManifest {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) kind: String,
    pub(in crate::appearance_store) description: Option<String>,
    #[serde(default)]
    pub(in crate::appearance_store) name_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(in crate::appearance_store) permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginSettingManifest {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) label: String,
    pub(in crate::appearance_store) description: Option<String>,
    #[serde(default)]
    pub(in crate::appearance_store) label_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) kind: String,
    pub(in crate::appearance_store) placement: String,
    pub(in crate::appearance_store) default_value: Value,
    pub(in crate::appearance_store) min: Option<f64>,
    pub(in crate::appearance_store) max: Option<f64>,
    pub(in crate::appearance_store) step: Option<f64>,
    #[serde(default)]
    pub(in crate::appearance_store) options: Vec<PluginSettingOption>,
    pub(in crate::appearance_store) mpv_property: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginActionManifest {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) label: String,
    pub(in crate::appearance_store) description: Option<String>,
    #[serde(default)]
    pub(in crate::appearance_store) label_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) placement: String,
    pub(in crate::appearance_store) command: String,
    pub(in crate::appearance_store) icon: Option<String>,
    #[serde(default)]
    pub(in crate::appearance_store) requires_media: bool,
    #[serde(default = "default_plugin_action_args")]
    pub(in crate::appearance_store) args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct PluginViewManifest {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) title: String,
    pub(in crate::appearance_store) entry: String,
    pub(in crate::appearance_store) description: Option<String>,
    #[serde(default = "default_plugin_view_presentation")]
    pub(in crate::appearance_store) presentation: String,
    pub(in crate::appearance_store) frame_opacity_setting: Option<String>,
    #[serde(default)]
    pub(in crate::appearance_store) title_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
}

fn default_plugin_view_presentation() -> String {
    "overlay".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingOption {
    pub(in crate::appearance_store) value: String,
    pub(in crate::appearance_store) label: String,
    #[serde(default)]
    pub(in crate::appearance_store) label_i18n: HashMap<String, String>,
}
