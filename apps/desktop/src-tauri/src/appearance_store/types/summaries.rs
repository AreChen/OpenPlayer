use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::PluginSettingOption;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePluginSummary {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) version: String,
    pub(in crate::appearance_store) api_version: String,
    pub(in crate::appearance_store) min_host_version: Option<String>,
    pub(in crate::appearance_store) author: Option<String>,
    pub(in crate::appearance_store) update_url: Option<String>,
    pub(in crate::appearance_store) description: Option<String>,
    pub(in crate::appearance_store) enabled: bool,
    pub(in crate::appearance_store) package_kind: String,
    pub(in crate::appearance_store) install_path: Option<String>,
    pub(in crate::appearance_store) installed_at_ms: Option<u64>,
    pub(in crate::appearance_store) theme_count: usize,
    pub(in crate::appearance_store) runtime: String,
    pub(in crate::appearance_store) capability_count: usize,
    pub(in crate::appearance_store) setting_count: usize,
    pub(in crate::appearance_store) action_count: usize,
    pub(in crate::appearance_store) permissions: Vec<String>,
    pub(in crate::appearance_store) capabilities: Vec<PluginCapabilitySummary>,
    pub(in crate::appearance_store) settings: Vec<PluginSettingSummary>,
    pub(in crate::appearance_store) actions: Vec<PluginActionSummary>,
    pub(in crate::appearance_store) views: Vec<PluginViewSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilitySummary {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) kind: String,
    pub(in crate::appearance_store) description: Option<String>,
    pub(in crate::appearance_store) name_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingSummary {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) label: String,
    pub(in crate::appearance_store) description: Option<String>,
    pub(in crate::appearance_store) label_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) kind: String,
    pub(in crate::appearance_store) placement: String,
    pub(in crate::appearance_store) default_value: Value,
    pub(in crate::appearance_store) value: Value,
    pub(in crate::appearance_store) min: Option<f64>,
    pub(in crate::appearance_store) max: Option<f64>,
    pub(in crate::appearance_store) step: Option<f64>,
    pub(in crate::appearance_store) options: Vec<PluginSettingOption>,
    pub(in crate::appearance_store) mpv_property: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionSummary {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) label: String,
    pub(in crate::appearance_store) description: Option<String>,
    pub(in crate::appearance_store) label_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) placement: String,
    pub(in crate::appearance_store) command: String,
    pub(in crate::appearance_store) icon: Option<String>,
    pub(in crate::appearance_store) requires_media: bool,
    pub(in crate::appearance_store) args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginViewSummary {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) title: String,
    pub(in crate::appearance_store) entry: String,
    pub(in crate::appearance_store) description: Option<String>,
    pub(in crate::appearance_store) presentation: String,
    pub(in crate::appearance_store) frame_opacity_setting: Option<String>,
    pub(in crate::appearance_store) title_i18n: HashMap<String, String>,
    pub(in crate::appearance_store) description_i18n: HashMap<String, String>,
}
