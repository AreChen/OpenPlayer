use std::fs;

use redb::{ReadableDatabase, ReadableTable};

use crate::appearance_store::{
    MAX_PLUGIN_RUNTIME_SCRIPT_BYTES, PLUGIN_ENABLEMENT, PLUGIN_INSTALLS, PLUGIN_MANIFESTS,
    package::resolve_plugin_runtime_script_path,
    records::{
        decode_plugin_manifest, plugin_enabled_from_table, plugin_install_from_table,
        plugin_permissions,
    },
    store::AppearanceStore,
    types::{PluginRuntimeKind, PluginRuntimeSource},
};

impl AppearanceStore {
    pub(in crate::appearance_store) fn plugin_runtime_sources(
        &self,
    ) -> Result<Vec<PluginRuntimeSource>, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin runtime sources: {error}"))?;
        let plugin_manifests = transaction
            .open_table(PLUGIN_MANIFESTS)
            .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
        let plugin_enablement = transaction
            .open_table(PLUGIN_ENABLEMENT)
            .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
        let plugin_installs = transaction
            .open_table(PLUGIN_INSTALLS)
            .map_err(|error| format!("failed to open plugin installs table: {error}"))?;

        let mut runtime_sources = Vec::new();
        for item in plugin_manifests
            .iter()
            .map_err(|error| format!("failed to scan plugin manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read plugin manifest: {error}"))?;
            let manifest = decode_plugin_manifest(value.value())?;
            if manifest.runtime.kind != PluginRuntimeKind::WebviewJs
                || !plugin_enabled_from_table(&plugin_enablement, &manifest.id)?
            {
                continue;
            }

            let install = plugin_install_from_table(&plugin_installs, &manifest.id)?
                .ok_or_else(|| format!("runtime plugin {} is not installed", manifest.id))?;
            let entry = manifest
                .runtime
                .entry
                .as_deref()
                .ok_or_else(|| format!("runtime plugin {} is missing an entry", manifest.id))?;
            let script_path = resolve_plugin_runtime_script_path(&install.install_path, entry)?;
            let metadata = fs::metadata(&script_path)
                .map_err(|error| format!("failed to inspect plugin runtime script: {error}"))?;
            if metadata.len() > MAX_PLUGIN_RUNTIME_SCRIPT_BYTES {
                return Err(format!("plugin runtime script is too large: {entry}"));
            }
            let script = fs::read_to_string(&script_path)
                .map_err(|error| format!("failed to read plugin runtime script: {error}"))?;

            runtime_sources.push(PluginRuntimeSource {
                plugin_id: manifest.id.clone(),
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                entry: entry.to_string(),
                script,
                permissions: plugin_permissions(&manifest),
            });
        }
        runtime_sources.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.plugin_id.cmp(&right.plugin_id))
        });
        Ok(runtime_sources)
    }
}
