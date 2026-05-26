use redb::ReadableTable;
use serde_json::Value;

use super::{
    ACTIVE_THEME_KEY, DEFAULT_THEME_ID, PLUGIN_ENABLEMENT, PLUGIN_MANIFESTS, PLUGIN_SETTINGS,
    SETTINGS_KV, THEME_MANIFESTS,
    manifest::validate_plugin_setting_value,
    records::{plugin_setting_key, theme_belongs_to_plugin},
    store::AppearanceStore,
    types::AppearanceState,
};

impl AppearanceStore {
    pub(super) fn set_plugin_enabled(
        &mut self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<AppearanceState, String> {
        let plugin_id = plugin_id.trim();
        let active_theme_id = self.state()?.active_theme_id;
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin enablement: {error}"))?;
        {
            let plugin_manifests = transaction
                .open_table(PLUGIN_MANIFESTS)
                .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
            if plugin_manifests
                .get(plugin_id)
                .map_err(|error| format!("failed to read plugin manifest: {error}"))?
                .is_none()
            {
                return Err(format!("unknown theme plugin: {plugin_id}"));
            }

            let mut plugin_enablement = transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
            plugin_enablement
                .insert(plugin_id, if enabled { "true" } else { "false" })
                .map_err(|error| format!("failed to store plugin enablement: {error}"))?;

            if !enabled {
                let theme_manifests = transaction
                    .open_table(THEME_MANIFESTS)
                    .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
                if theme_belongs_to_plugin(&theme_manifests, &active_theme_id, plugin_id)? {
                    let mut settings = transaction.open_table(SETTINGS_KV).map_err(|error| {
                        format!("failed to open appearance settings table: {error}")
                    })?;
                    settings
                        .insert(ACTIVE_THEME_KEY, DEFAULT_THEME_ID)
                        .map_err(|error| {
                            format!("failed to store active theme fallback: {error}")
                        })?;
                }
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin enablement: {error}"))?;
        self.state()
    }

    pub(super) fn set_plugin_setting(
        &mut self,
        plugin_id: &str,
        setting_id: &str,
        value: Value,
    ) -> Result<AppearanceState, String> {
        let plugin_id = plugin_id.trim();
        let setting_id = setting_id.trim();
        let manifest = self.plugin_manifest(plugin_id)?;
        let setting = manifest
            .contributes
            .settings
            .iter()
            .find(|setting| setting.id == setting_id)
            .ok_or_else(|| format!("unknown plugin setting: {plugin_id}.{setting_id}"))?;
        validate_plugin_setting_value(setting, &value)?;
        let encoded = serde_json::to_string(&value)
            .map_err(|error| format!("failed to encode plugin setting value: {error}"))?;
        let key = plugin_setting_key(plugin_id, setting_id);

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin setting: {error}"))?;
        {
            let mut settings = transaction
                .open_table(PLUGIN_SETTINGS)
                .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
            settings
                .insert(key.as_str(), encoded.as_str())
                .map_err(|error| format!("failed to store plugin setting: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin setting: {error}"))?;
        self.state()
    }
}
