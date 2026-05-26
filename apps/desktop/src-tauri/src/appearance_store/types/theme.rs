use serde::{Deserialize, Serialize};

use super::ThemePluginSummary;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTokens {
    pub(in crate::appearance_store) surface: String,
    pub(in crate::appearance_store) panel: String,
    pub(in crate::appearance_store) panel_strong: String,
    pub(in crate::appearance_store) text: String,
    pub(in crate::appearance_store) muted: String,
    pub(in crate::appearance_store) faint: String,
    pub(in crate::appearance_store) accent: String,
    pub(in crate::appearance_store) danger: String,
    pub(in crate::appearance_store) line: String,
    pub(in crate::appearance_store) control: String,
    pub(in crate::appearance_store) scrollbar_thumb: String,
    pub(in crate::appearance_store) scrollbar_thumb_hover: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct ThemeManifest {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) version: String,
    pub(in crate::appearance_store) tokens: ThemeTokens,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::appearance_store) struct StoredThemeManifest {
    pub(in crate::appearance_store) plugin_id: String,
    pub(in crate::appearance_store) theme: ThemeManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeCatalogItem {
    pub(in crate::appearance_store) id: String,
    pub(in crate::appearance_store) name: String,
    pub(in crate::appearance_store) version: String,
    pub(in crate::appearance_store) source: String,
    pub(in crate::appearance_store) plugin_id: Option<String>,
    pub(in crate::appearance_store) enabled: bool,
    pub(in crate::appearance_store) tokens: ThemeTokens,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub(in crate::appearance_store) active_theme_id: String,
    pub(in crate::appearance_store) accent_override: Option<String>,
    pub(in crate::appearance_store) themes: Vec<ThemeCatalogItem>,
    pub(in crate::appearance_store) plugins: Vec<ThemePluginSummary>,
}
