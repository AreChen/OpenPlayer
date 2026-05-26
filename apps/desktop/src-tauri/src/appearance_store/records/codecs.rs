use serde_json::Value;

use crate::appearance_store::types::{
    PluginManifest, PluginRuntimeKind, StoredPluginInstall, StoredThemeManifest,
};

pub(in crate::appearance_store) fn decode_plugin_setting_value(
    value: &str,
) -> Result<Value, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode plugin setting: {error}"))
}

pub(in crate::appearance_store) fn decode_plugin_runtime_storage_value(
    value: &str,
) -> Result<Value, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin runtime storage value: {error}"))
}

pub(in crate::appearance_store) fn runtime_kind_label(kind: &PluginRuntimeKind) -> &'static str {
    match kind {
        PluginRuntimeKind::Manifest => "manifest",
        PluginRuntimeKind::WebviewJs => "webviewJs",
        PluginRuntimeKind::Wasm => "wasm",
    }
}

pub(in crate::appearance_store) fn decode_plugin_manifest(
    value: &str,
) -> Result<PluginManifest, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin manifest: {error}"))
}

pub(in crate::appearance_store) fn decode_plugin_install_record(
    value: &str,
) -> Result<StoredPluginInstall, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin install record: {error}"))
}

pub(in crate::appearance_store) fn decode_stored_theme_manifest(
    value: &str,
) -> Result<StoredThemeManifest, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode theme manifest: {error}"))
}
