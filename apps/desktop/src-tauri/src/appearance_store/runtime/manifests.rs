use redb::ReadableDatabase;

use crate::appearance_store::{
    PLUGIN_MANIFESTS, records::decode_plugin_manifest, store::AppearanceStore,
    types::PluginManifest,
};

impl AppearanceStore {
    pub(in crate::appearance_store) fn plugin_manifest(
        &self,
        plugin_id: &str,
    ) -> Result<PluginManifest, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin manifest: {error}"))?;
        let plugin_manifests = transaction
            .open_table(PLUGIN_MANIFESTS)
            .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
        let stored = plugin_manifests
            .get(plugin_id)
            .map_err(|error| format!("failed to read plugin manifest: {error}"))?
            .ok_or_else(|| format!("unknown plugin: {plugin_id}"))?;
        decode_plugin_manifest(stored.value())
    }
}
