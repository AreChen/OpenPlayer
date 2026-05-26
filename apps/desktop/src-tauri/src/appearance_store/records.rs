use std::time::{SystemTime, UNIX_EPOCH};

use redb::ReadableTable;
use serde_json::Value;

use super::{
    LANGUAGE_MODE_KEY, MAX_PLUGIN_RUNTIME_STORAGE_KEY_BYTES,
    manifest::{is_allowed_plugin_mpv_property, validate_plugin_setting_value},
    types::*,
};
pub(super) fn plugin_enabled_from_table<T>(table: &T, plugin_id: &str) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    Ok(table
        .get(plugin_id)
        .map_err(|error| format!("failed to read plugin enablement: {error}"))?
        .map(|value| value.value() != "false")
        .unwrap_or(true))
}

pub(super) fn plugin_setting_summaries<T>(
    table: &T,
    manifest: &PluginManifest,
) -> Result<Vec<PluginSettingSummary>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let mut settings = Vec::new();
    for setting in &manifest.contributes.settings {
        if let Some(property) = setting.mpv_property.as_deref()
            && !is_allowed_plugin_mpv_property(property)
        {
            continue;
        }
        let key = plugin_setting_key(&manifest.id, &setting.id);
        let stored_value = table
            .get(key.as_str())
            .map_err(|error| format!("failed to read plugin setting: {error}"))?
            .map(|stored| decode_plugin_setting_value(stored.value()))
            .transpose()?;
        let value = stored_value
            .filter(|value| validate_plugin_setting_value(setting, value).is_ok())
            .unwrap_or_else(|| setting.default_value.clone());
        settings.push(PluginSettingSummary {
            id: setting.id.clone(),
            label: setting.label.clone(),
            description: setting.description.clone(),
            label_i18n: setting.label_i18n.clone(),
            description_i18n: setting.description_i18n.clone(),
            kind: setting.kind.clone(),
            placement: setting.placement.clone(),
            default_value: setting.default_value.clone(),
            value,
            min: setting.min,
            max: setting.max,
            step: setting.step,
            options: setting.options.clone(),
            mpv_property: setting.mpv_property.clone(),
        });
    }
    Ok(settings)
}

pub(super) fn plugin_capability_summaries(
    manifest: &PluginManifest,
) -> Vec<PluginCapabilitySummary> {
    manifest
        .contributes
        .capabilities
        .iter()
        .map(|capability| PluginCapabilitySummary {
            id: capability.id.clone(),
            name: capability.name.clone(),
            kind: capability.kind.clone(),
            description: capability.description.clone(),
            name_i18n: capability.name_i18n.clone(),
            description_i18n: capability.description_i18n.clone(),
            permissions: capability.permissions.clone(),
        })
        .collect()
}

pub(super) fn plugin_action_summaries(manifest: &PluginManifest) -> Vec<PluginActionSummary> {
    manifest
        .contributes
        .actions
        .iter()
        .map(|action| PluginActionSummary {
            id: action.id.clone(),
            label: action.label.clone(),
            description: action.description.clone(),
            label_i18n: action.label_i18n.clone(),
            description_i18n: action.description_i18n.clone(),
            placement: action.placement.clone(),
            command: action.command.clone(),
            icon: action.icon.clone(),
            requires_media: action.requires_media,
            args: action.args.clone(),
        })
        .collect()
}

pub(super) fn plugin_view_summaries(manifest: &PluginManifest) -> Vec<PluginViewSummary> {
    manifest
        .contributes
        .views
        .iter()
        .map(|view| PluginViewSummary {
            id: view.id.clone(),
            title: view.title.clone(),
            entry: view.entry.clone(),
            description: view.description.clone(),
            title_i18n: view.title_i18n.clone(),
            description_i18n: view.description_i18n.clone(),
        })
        .collect()
}

pub(super) fn plugin_permissions(manifest: &PluginManifest) -> Vec<String> {
    let mut permissions: Vec<String> = manifest
        .contributes
        .capabilities
        .iter()
        .flat_map(|capability| capability.permissions.iter().cloned())
        .collect();
    permissions.sort();
    permissions.dedup();
    permissions
}

pub(super) fn plugin_install_from_table<T>(
    table: &T,
    plugin_id: &str,
) -> Result<Option<StoredPluginInstall>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    table
        .get(plugin_id)
        .map_err(|error| format!("failed to read plugin install record: {error}"))?
        .map(|value| decode_plugin_install_record(value.value()))
        .transpose()
}

pub(super) fn plugin_setting_key(plugin_id: &str, setting_id: &str) -> String {
    format!("{plugin_id}::{setting_id}")
}

pub(super) fn plugin_runtime_storage_prefix(plugin_id: &str) -> String {
    format!("{plugin_id}::")
}

pub(super) fn plugin_runtime_storage_key(plugin_id: &str, key: &str) -> String {
    format!("{}{key}", plugin_runtime_storage_prefix(plugin_id))
}

pub(super) fn validate_plugin_runtime_storage_key(key: &str) -> Result<&str, String> {
    let key = key.trim();
    if key.is_empty()
        || key.len() > MAX_PLUGIN_RUNTIME_STORAGE_KEY_BYTES
        || key.starts_with('.')
        || key.ends_with('.')
        || key.contains("..")
        || !key
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | '-'))
    {
        return Err(format!("plugin runtime storage key is invalid: {key}"));
    }
    Ok(key)
}

pub(super) fn plugin_setting_keys_for_plugin(
    table: &redb::Table<'_, &str, &str>,
    plugin_id: &str,
) -> Result<Vec<String>, String> {
    let prefix = format!("{plugin_id}::");
    let mut keys = Vec::new();
    for item in table
        .iter()
        .map_err(|error| format!("failed to scan plugin settings: {error}"))?
    {
        let (key, _) = item.map_err(|error| format!("failed to read plugin setting: {error}"))?;
        if key.value().starts_with(&prefix) {
            keys.push(key.value().to_string());
        }
    }
    Ok(keys)
}

pub(super) fn plugin_runtime_storage_keys_for_plugin(
    table: &redb::Table<'_, &str, &str>,
    plugin_id: &str,
) -> Result<Vec<String>, String> {
    let prefix = plugin_runtime_storage_prefix(plugin_id);
    let mut keys = Vec::new();
    for item in table
        .iter()
        .map_err(|error| format!("failed to scan plugin runtime storage: {error}"))?
    {
        let (key, _) =
            item.map_err(|error| format!("failed to read plugin runtime storage: {error}"))?;
        if key.value().starts_with(&prefix) {
            keys.push(key.value().to_string());
        }
    }
    Ok(keys)
}

pub(super) fn decode_plugin_setting_value(value: &str) -> Result<Value, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode plugin setting: {error}"))
}

pub(super) fn decode_plugin_runtime_storage_value(value: &str) -> Result<Value, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin runtime storage value: {error}"))
}

pub(super) fn runtime_kind_label(kind: &PluginRuntimeKind) -> &'static str {
    match kind {
        PluginRuntimeKind::Manifest => "manifest",
        PluginRuntimeKind::WebviewJs => "webviewJs",
        PluginRuntimeKind::Wasm => "wasm",
    }
}

pub(super) fn read_bool_setting<T>(table: &T, key: &str) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    Ok(table
        .get(key)
        .map_err(|error| format!("failed to read boolean setting {key}: {error}"))?
        .map(|value| value.value() == "true")
        .unwrap_or(false))
}

pub(super) fn read_language_mode_setting<T>(table: &T) -> Result<String, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let Some(value) = table
        .get(LANGUAGE_MODE_KEY)
        .map_err(|error| format!("failed to read language preference: {error}"))?
    else {
        return Ok("system".to_string());
    };

    validate_language_mode(value.value()).map(ToOwned::to_owned)
}

pub(super) fn validate_language_mode(mode: &str) -> Result<&'static str, String> {
    match mode {
        "system" => Ok("system"),
        "en-US" => Ok("en-US"),
        "zh-CN" => Ok("zh-CN"),
        _ => Err("invalid language mode".to_string()),
    }
}

pub(super) fn theme_manifests_for_plugin(
    table: &redb::Table<'_, &str, &str>,
    plugin_id: &str,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for item in table
        .iter()
        .map_err(|error| format!("failed to scan theme manifests: {error}"))?
    {
        let (id, value) =
            item.map_err(|error| format!("failed to read theme manifest: {error}"))?;
        let stored = decode_stored_theme_manifest(value.value())?;
        if stored.plugin_id == plugin_id {
            ids.push(id.value().to_string());
        }
    }
    Ok(ids)
}

pub(super) fn theme_belongs_to_plugin<T>(
    table: &T,
    theme_id: &str,
    plugin_id: &str,
) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let Some(stored) = table
        .get(theme_id)
        .map_err(|error| format!("failed to read active theme manifest: {error}"))?
    else {
        return Ok(false);
    };
    Ok(decode_stored_theme_manifest(stored.value())?.plugin_id == plugin_id)
}

pub(super) fn decode_plugin_manifest(value: &str) -> Result<PluginManifest, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin manifest: {error}"))
}

pub(super) fn decode_plugin_install_record(value: &str) -> Result<StoredPluginInstall, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin install record: {error}"))
}

pub(super) fn decode_stored_theme_manifest(value: &str) -> Result<StoredThemeManifest, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode theme manifest: {error}"))
}

pub(super) fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
