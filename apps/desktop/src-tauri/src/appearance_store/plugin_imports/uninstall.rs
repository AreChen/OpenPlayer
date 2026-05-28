use std::path::PathBuf;

use redb::ReadableDatabase;

use crate::appearance_store::{
    ACTIVE_THEME_KEY, DEFAULT_THEME_ID, PLUGIN_ENABLEMENT, PLUGIN_INSTALLS, PLUGIN_MANIFESTS,
    PLUGIN_RUNTIME_STORAGE, PLUGIN_RUNTIME_STORAGE_META, PLUGIN_SETTINGS, SETTINGS_KV,
    THEME_MANIFESTS,
    manifest::validate_dotted_identifier,
    package::remove_installed_plugin_directory,
    records::{
        plugin_install_from_table, plugin_runtime_storage_keys_for_plugin,
        plugin_setting_keys_for_plugin, theme_belongs_to_plugin, theme_manifests_for_plugin,
    },
    store::AppearanceStore,
    types::{AppearanceState, StoredPluginInstall},
};

impl AppearanceStore {
    pub(in crate::appearance_store) fn uninstall_plugin(
        &mut self,
        plugin_id: &str,
    ) -> Result<AppearanceState, String> {
        let plugin_id = plugin_id.trim();
        validate_dotted_identifier("plugin id", plugin_id, true)?;
        let active_theme_id = self.state()?.active_theme_id;
        let install_path = self
            .plugin_install_record(plugin_id)?
            .map(|record| record.install_path);

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to uninstall plugin: {error}"))?;
        {
            let mut plugin_manifests = transaction
                .open_table(PLUGIN_MANIFESTS)
                .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
            if plugin_manifests
                .remove(plugin_id)
                .map_err(|error| format!("failed to remove plugin manifest: {error}"))?
                .is_none()
            {
                return Err(format!("unknown plugin: {plugin_id}"));
            }

            let mut plugin_enablement = transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
            plugin_enablement
                .remove(plugin_id)
                .map_err(|error| format!("failed to remove plugin enablement: {error}"))?;

            let mut plugin_installs = transaction
                .open_table(PLUGIN_INSTALLS)
                .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
            plugin_installs
                .remove(plugin_id)
                .map_err(|error| format!("failed to remove plugin install record: {error}"))?;

            let mut theme_manifests = transaction
                .open_table(THEME_MANIFESTS)
                .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
            let active_theme_belongs_to_plugin =
                theme_belongs_to_plugin(&theme_manifests, &active_theme_id, plugin_id)?;
            let stale_theme_ids = theme_manifests_for_plugin(&theme_manifests, plugin_id)?;
            for theme_id in stale_theme_ids {
                theme_manifests
                    .remove(theme_id.as_str())
                    .map_err(|error| format!("failed to remove plugin theme: {error}"))?;
            }

            let mut plugin_settings = transaction
                .open_table(PLUGIN_SETTINGS)
                .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
            let stale_setting_keys = plugin_setting_keys_for_plugin(&plugin_settings, plugin_id)?;
            for key in stale_setting_keys {
                plugin_settings
                    .remove(key.as_str())
                    .map_err(|error| format!("failed to remove plugin setting: {error}"))?;
            }

            let mut plugin_runtime_storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            let stale_storage_keys =
                plugin_runtime_storage_keys_for_plugin(&plugin_runtime_storage, plugin_id)?;
            for key in stale_storage_keys {
                plugin_runtime_storage
                    .remove(key.as_str())
                    .map_err(|error| {
                        format!("failed to remove plugin runtime storage value: {error}")
                    })?;
            }

            let mut plugin_runtime_storage_meta = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE_META)
                .map_err(|error| {
                    format!("failed to open plugin runtime storage metadata table: {error}")
                })?;
            plugin_runtime_storage_meta
                .remove(plugin_id)
                .map_err(|error| {
                    format!("failed to remove plugin runtime storage metadata: {error}")
                })?;

            if active_theme_belongs_to_plugin {
                let mut settings = transaction.open_table(SETTINGS_KV).map_err(|error| {
                    format!("failed to open appearance settings table: {error}")
                })?;
                settings
                    .insert(ACTIVE_THEME_KEY, DEFAULT_THEME_ID)
                    .map_err(|error| format!("failed to store active theme fallback: {error}"))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin uninstall: {error}"))?;

        if let Some(install_path) = install_path {
            remove_installed_plugin_directory(&self.plugin_root, &PathBuf::from(install_path))?;
        }
        self.state()
    }

    pub(in crate::appearance_store) fn plugin_install_record(
        &self,
        plugin_id: &str,
    ) -> Result<Option<StoredPluginInstall>, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin install record: {error}"))?;
        let plugin_installs = transaction
            .open_table(PLUGIN_INSTALLS)
            .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
        plugin_install_from_table(&plugin_installs, plugin_id)
    }
}
