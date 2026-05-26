use redb::ReadableTable;

use crate::appearance_store::MAX_PLUGIN_RUNTIME_STORAGE_KEY_BYTES;

pub(in crate::appearance_store) fn plugin_setting_key(plugin_id: &str, setting_id: &str) -> String {
    format!("{plugin_id}::{setting_id}")
}

pub(in crate::appearance_store) fn plugin_runtime_storage_prefix(plugin_id: &str) -> String {
    format!("{plugin_id}::")
}

pub(in crate::appearance_store) fn plugin_runtime_storage_key(
    plugin_id: &str,
    key: &str,
) -> String {
    format!("{}{key}", plugin_runtime_storage_prefix(plugin_id))
}

pub(in crate::appearance_store) fn validate_plugin_runtime_storage_key(
    key: &str,
) -> Result<&str, String> {
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

pub(in crate::appearance_store) fn plugin_setting_keys_for_plugin(
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

pub(in crate::appearance_store) fn plugin_runtime_storage_keys_for_plugin(
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
