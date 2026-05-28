use std::collections::HashMap;

use redb::{ReadableDatabase, ReadableTable};
use serde_json::Value;

use crate::appearance_store::{
    MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES, PLUGIN_RUNTIME_STORAGE, PLUGIN_RUNTIME_STORAGE_META,
    records::{
        decode_plugin_runtime_storage_value, plugin_runtime_storage_key,
        plugin_runtime_storage_prefix, validate_plugin_runtime_storage_key,
    },
    store::AppearanceStore,
    types::PluginStorageInfo,
};

impl AppearanceStore {
    pub(in crate::appearance_store) fn mark_plugin_runtime_storage_migrated(
        &mut self,
        plugin_id: &str,
        schema_version: Option<u32>,
    ) -> Result<PluginStorageInfo, String> {
        let plugin_id = plugin_id.trim();
        let manifest = self.plugin_manifest(plugin_id)?;
        let manifest_version = manifest
            .contributes
            .storage
            .as_ref()
            .map(|storage| storage.version)
            .unwrap_or(0);
        let target_version = schema_version.unwrap_or(manifest_version);
        if target_version == 0 {
            return Err("plugin does not declare a storage schema".to_string());
        }
        if target_version > manifest_version {
            return Err(format!(
                "plugin storage migration target {target_version} exceeds manifest version {manifest_version}"
            ));
        }

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin runtime storage metadata: {error}"))?;
        {
            let mut storage_meta = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE_META)
                .map_err(|error| {
                    format!("failed to open plugin runtime storage metadata table: {error}")
                })?;
            let current_version = storage_meta
                .get(plugin_id)
                .map_err(|error| {
                    format!("failed to read plugin runtime storage metadata: {error}")
                })?
                .map(|value| parse_plugin_runtime_storage_schema_version(value.value()))
                .transpose()?
                .unwrap_or(0);
            if target_version < current_version {
                return Err(format!(
                    "plugin storage migration target {target_version} is older than current schema version {current_version}"
                ));
            }
            let encoded_version = target_version.to_string();
            storage_meta
                .insert(plugin_id, encoded_version.as_str())
                .map_err(|error| {
                    format!("failed to store plugin runtime storage metadata: {error}")
                })?;
        }
        transaction.commit().map_err(|error| {
            format!("failed to commit plugin runtime storage metadata: {error}")
        })?;

        self.plugin_runtime_storage_info(plugin_id)
    }

    pub(in crate::appearance_store) fn plugin_runtime_storage_info(
        &self,
        plugin_id: &str,
    ) -> Result<PluginStorageInfo, String> {
        let plugin_id = plugin_id.trim();
        let manifest = self.plugin_manifest(plugin_id)?;
        let prefix = plugin_runtime_storage_prefix(plugin_id);
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin runtime storage info: {error}"))?;
        let storage = transaction
            .open_table(PLUGIN_RUNTIME_STORAGE)
            .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
        let storage_meta = transaction
            .open_table(PLUGIN_RUNTIME_STORAGE_META)
            .map_err(|error| {
                format!("failed to open plugin runtime storage metadata table: {error}")
            })?;
        let mut keys = Vec::new();
        for item in storage
            .iter()
            .map_err(|error| format!("failed to scan plugin runtime storage: {error}"))?
        {
            let (key, _) =
                item.map_err(|error| format!("failed to read plugin runtime storage: {error}"))?;
            if let Some(item_key) = key.value().strip_prefix(&prefix) {
                keys.push(item_key.to_string());
            }
        }
        keys.sort();
        let schema_version = storage_meta
            .get(plugin_id)
            .map_err(|error| format!("failed to read plugin runtime storage metadata: {error}"))?
            .map(|value| parse_plugin_runtime_storage_schema_version(value.value()))
            .transpose()?
            .unwrap_or(0);
        let manifest_version = manifest
            .contributes
            .storage
            .as_ref()
            .map(|storage| storage.version)
            .unwrap_or(0);

        Ok(PluginStorageInfo {
            plugin_id: plugin_id.to_string(),
            schema_version,
            manifest_version,
            keys,
        })
    }

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

    pub(in crate::appearance_store) fn update_plugin_runtime_storage_values(
        &mut self,
        plugin_id: &str,
        set: HashMap<String, Value>,
        remove: Vec<String>,
    ) -> Result<PluginStorageInfo, String> {
        let plugin_id = plugin_id.trim();
        self.plugin_manifest(plugin_id)?;

        let mut encoded_set = Vec::new();
        for (key, value) in set {
            let key = validate_plugin_runtime_storage_key(&key)?.to_string();
            let encoded = serde_json::to_string(&value).map_err(|error| {
                format!("failed to encode plugin runtime storage value: {error}")
            })?;
            if encoded.len() > MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES {
                return Err("plugin runtime storage value is too large".to_string());
            }
            encoded_set.push((plugin_runtime_storage_key(plugin_id, &key), encoded));
        }

        let mut remove_keys = Vec::new();
        for key in remove {
            let key = validate_plugin_runtime_storage_key(&key)?.to_string();
            remove_keys.push(plugin_runtime_storage_key(plugin_id, &key));
        }

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to update plugin runtime storage: {error}"))?;
        {
            let mut storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            for key in remove_keys {
                storage.remove(key.as_str()).map_err(|error| {
                    format!("failed to remove plugin runtime storage value: {error}")
                })?;
            }
            for (key, encoded) in encoded_set {
                storage
                    .insert(key.as_str(), encoded.as_str())
                    .map_err(|error| {
                        format!("failed to store plugin runtime storage value: {error}")
                    })?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin runtime storage update: {error}"))?;

        self.plugin_runtime_storage_info(plugin_id)
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

pub(in crate::appearance_store) fn parse_plugin_runtime_storage_schema_version(
    value: &str,
) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("failed to decode plugin runtime storage metadata: {error}"))
}
