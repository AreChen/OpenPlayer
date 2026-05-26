use std::collections::HashMap;

use redb::{ReadableDatabase, ReadableTable};
use serde_json::Value;

use crate::appearance_store::{
    MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES, PLUGIN_RUNTIME_STORAGE,
    records::{
        decode_plugin_runtime_storage_value, plugin_runtime_storage_key,
        plugin_runtime_storage_prefix, validate_plugin_runtime_storage_key,
    },
    store::AppearanceStore,
};

impl AppearanceStore {
    pub(in crate::appearance_store) fn plugin_runtime_storage_value(
        &self,
        plugin_id: &str,
        key: &str,
    ) -> Result<Option<Value>, String> {
        let plugin_id = plugin_id.trim();
        let key = validate_plugin_runtime_storage_key(key)?;
        self.plugin_manifest(plugin_id)?;

        let storage_key = plugin_runtime_storage_key(plugin_id, key);
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin runtime storage: {error}"))?;
        let storage = transaction
            .open_table(PLUGIN_RUNTIME_STORAGE)
            .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
        storage
            .get(storage_key.as_str())
            .map_err(|error| format!("failed to read plugin runtime storage value: {error}"))?
            .map(|value| decode_plugin_runtime_storage_value(value.value()))
            .transpose()
    }

    pub(in crate::appearance_store) fn plugin_runtime_storage_values(
        &self,
        plugin_id: &str,
    ) -> Result<HashMap<String, Value>, String> {
        let plugin_id = plugin_id.trim();
        let prefix = plugin_runtime_storage_prefix(plugin_id);
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to list plugin runtime storage: {error}"))?;
        let storage = transaction
            .open_table(PLUGIN_RUNTIME_STORAGE)
            .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
        let mut values = HashMap::new();
        for item in storage
            .iter()
            .map_err(|error| format!("failed to scan plugin runtime storage: {error}"))?
        {
            let (key, value) =
                item.map_err(|error| format!("failed to read plugin runtime storage: {error}"))?;
            if let Some(item_key) = key.value().strip_prefix(&prefix) {
                values.insert(
                    item_key.to_string(),
                    decode_plugin_runtime_storage_value(value.value())?,
                );
            }
        }
        Ok(values)
    }

    pub(in crate::appearance_store) fn set_plugin_runtime_storage_value(
        &mut self,
        plugin_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let plugin_id = plugin_id.trim();
        let key = validate_plugin_runtime_storage_key(key)?;
        self.plugin_manifest(plugin_id)?;
        let encoded = serde_json::to_string(&value)
            .map_err(|error| format!("failed to encode plugin runtime storage value: {error}"))?;
        if encoded.len() > MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES {
            return Err("plugin runtime storage value is too large".to_string());
        }

        let storage_key = plugin_runtime_storage_key(plugin_id, key);
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin runtime storage: {error}"))?;
        {
            let mut storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            storage
                .insert(storage_key.as_str(), encoded.as_str())
                .map_err(|error| {
                    format!("failed to store plugin runtime storage value: {error}")
                })?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin runtime storage: {error}"))
    }

    pub(in crate::appearance_store) fn remove_plugin_runtime_storage_value(
        &mut self,
        plugin_id: &str,
        key: &str,
    ) -> Result<bool, String> {
        let plugin_id = plugin_id.trim();
        let key = validate_plugin_runtime_storage_key(key)?;
        self.plugin_manifest(plugin_id)?;
        let storage_key = plugin_runtime_storage_key(plugin_id, key);
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to remove plugin runtime storage: {error}"))?;
        let removed = {
            let mut storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            storage
                .remove(storage_key.as_str())
                .map_err(|error| format!("failed to remove plugin runtime storage value: {error}"))?
                .is_some()
        };
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin runtime storage removal: {error}"))?;
        Ok(removed)
    }
}
