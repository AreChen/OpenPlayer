use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::SUPPORTED_PLUGIN_API_VERSION;
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTokens {
    pub(super) surface: String,
    pub(super) panel: String,
    pub(super) panel_strong: String,
    pub(super) text: String,
    pub(super) muted: String,
    pub(super) faint: String,
    pub(super) accent: String,
    pub(super) danger: String,
    pub(super) line: String,
    pub(super) control: String,
    pub(super) scrollbar_thumb: String,
    pub(super) scrollbar_thumb_hover: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ThemeManifest {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) tokens: ThemeTokens,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StoredThemeManifest {
    pub(super) plugin_id: String,
    pub(super) theme: ThemeManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StoredPluginInstall {
    pub(super) package_kind: String,
    pub(super) install_path: String,
    pub(super) installed_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginManifest {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) version: String,
    #[serde(default = "default_plugin_api_version")]
    pub(super) api_version: String,
    pub(super) min_host_version: Option<String>,
    pub(super) author: Option<String>,
    pub(super) update_url: Option<String>,
    pub(super) description: Option<String>,
    pub(super) entry: ThemePluginEntry,
    #[serde(default)]
    pub(super) runtime: PluginRuntime,
    pub(super) contributes: PluginContributions,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginContributions {
    #[serde(default)]
    pub(super) themes: Vec<ThemeManifest>,
    #[serde(default)]
    pub(super) capabilities: Vec<PluginCapabilityManifest>,
    #[serde(default)]
    pub(super) settings: Vec<PluginSettingManifest>,
    #[serde(default)]
    pub(super) actions: Vec<PluginActionManifest>,
    #[serde(default)]
    pub(super) views: Vec<PluginViewManifest>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) enum ThemePluginEntry {
    #[serde(rename = "manifest")]
    Manifest,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginRuntime {
    pub(super) kind: PluginRuntimeKind,
    pub(super) entry: Option<String>,
    pub(super) sandbox: Option<String>,
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self {
            kind: PluginRuntimeKind::Manifest,
            entry: None,
            sandbox: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) enum PluginRuntimeKind {
    #[serde(rename = "manifest")]
    Manifest,
    #[serde(rename = "webviewJs")]
    WebviewJs,
    #[serde(rename = "wasm")]
    Wasm,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginCapabilityManifest {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) name_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(super) description_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(super) permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginSettingManifest {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) label_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(super) description_i18n: HashMap<String, String>,
    pub(super) kind: String,
    pub(super) placement: String,
    pub(super) default_value: Value,
    pub(super) min: Option<f64>,
    pub(super) max: Option<f64>,
    pub(super) step: Option<f64>,
    #[serde(default)]
    pub(super) options: Vec<PluginSettingOption>,
    pub(super) mpv_property: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginActionManifest {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) label_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(super) description_i18n: HashMap<String, String>,
    pub(super) placement: String,
    pub(super) command: String,
    pub(super) icon: Option<String>,
    #[serde(default)]
    pub(super) requires_media: bool,
    #[serde(default = "default_plugin_action_args")]
    pub(super) args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PluginViewManifest {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) entry: String,
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) title_i18n: HashMap<String, String>,
    #[serde(default)]
    pub(super) description_i18n: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginSettingOption {
    pub(super) value: String,
    pub(super) label: String,
    #[serde(default)]
    pub(super) label_i18n: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeCatalogItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) source: String,
    pub(super) plugin_id: Option<String>,
    pub(super) enabled: bool,
    pub(super) tokens: ThemeTokens,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePluginSummary {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) api_version: String,
    pub(super) min_host_version: Option<String>,
    pub(super) author: Option<String>,
    pub(super) update_url: Option<String>,
    pub(super) description: Option<String>,
    pub(super) enabled: bool,
    pub(super) package_kind: String,
    pub(super) install_path: Option<String>,
    pub(super) installed_at_ms: Option<u64>,
    pub(super) theme_count: usize,
    pub(super) runtime: String,
    pub(super) capability_count: usize,
    pub(super) setting_count: usize,
    pub(super) action_count: usize,
    pub(super) permissions: Vec<String>,
    pub(super) capabilities: Vec<PluginCapabilitySummary>,
    pub(super) settings: Vec<PluginSettingSummary>,
    pub(super) actions: Vec<PluginActionSummary>,
    pub(super) views: Vec<PluginViewSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilitySummary {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) description: Option<String>,
    pub(super) name_i18n: HashMap<String, String>,
    pub(super) description_i18n: HashMap<String, String>,
    pub(super) permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingSummary {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) description: Option<String>,
    pub(super) label_i18n: HashMap<String, String>,
    pub(super) description_i18n: HashMap<String, String>,
    pub(super) kind: String,
    pub(super) placement: String,
    pub(super) default_value: Value,
    pub(super) value: Value,
    pub(super) min: Option<f64>,
    pub(super) max: Option<f64>,
    pub(super) step: Option<f64>,
    pub(super) options: Vec<PluginSettingOption>,
    pub(super) mpv_property: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionSummary {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) description: Option<String>,
    pub(super) label_i18n: HashMap<String, String>,
    pub(super) description_i18n: HashMap<String, String>,
    pub(super) placement: String,
    pub(super) command: String,
    pub(super) icon: Option<String>,
    pub(super) requires_media: bool,
    pub(super) args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginViewSummary {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) entry: String,
    pub(super) description: Option<String>,
    pub(super) title_i18n: HashMap<String, String>,
    pub(super) description_i18n: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeSource {
    pub(super) plugin_id: String,
    pub(super) name: String,
    pub(super) version: String,
    pub(super) entry: String,
    pub(super) script: String,
    pub(super) permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginViewHtml {
    pub(super) plugin_id: String,
    pub(super) view_id: String,
    pub(super) title: String,
    pub(super) html: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub(super) active_theme_id: String,
    pub(super) accent_override: Option<String>,
    pub(super) themes: Vec<ThemeCatalogItem>,
    pub(super) plugins: Vec<ThemePluginSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPreferences {
    pub(super) incognito_mode: bool,
    pub(super) quiet_keyboard_controls: bool,
    pub(super) language_mode: String,
}

pub(super) fn default_plugin_action_args() -> Value {
    serde_json::json!({})
}

pub(super) fn default_plugin_api_version() -> String {
    SUPPORTED_PLUGIN_API_VERSION.to_string()
}
