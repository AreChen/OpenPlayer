use crate::appearance_store::{
    MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES, records::validate_plugin_runtime_storage_key,
    types::PluginStorageManifest,
};

const MAX_PLUGIN_STORAGE_DEFAULT_KEYS: usize = 256;

pub(super) fn validate_plugin_storage(storage: &PluginStorageManifest) -> Result<(), String> {
    if storage.version == 0 {
        return Err("plugin storage version must be at least 1".to_string());
    }
    if storage.defaults.len() > MAX_PLUGIN_STORAGE_DEFAULT_KEYS {
        return Err("plugin storage defaults define too many keys".to_string());
    }

    for (key, value) in &storage.defaults {
        validate_plugin_runtime_storage_key(key)?;
        let encoded = serde_json::to_string(value)
            .map_err(|error| format!("plugin storage default {key} is invalid: {error}"))?;
        if encoded.len() > MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES {
            return Err(format!("plugin storage default {key} is too large"));
        }
    }

    Ok(())
}
