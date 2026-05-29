use redb::{ReadableDatabase, ReadableTable};

use super::{
    ACCENT_OVERRIDE_KEY, ACTIVE_THEME_KEY, DEFAULT_THEME_ID, PLUGIN_ENABLEMENT, PLUGIN_INSTALLS,
    PLUGIN_MANIFESTS, PLUGIN_SETTINGS, SETTINGS_KV, THEME_MANIFESTS,
    records::{
        decode_plugin_manifest, decode_stored_theme_manifest, plugin_action_summaries,
        plugin_capability_summaries, plugin_enabled_from_table, plugin_install_from_table,
        plugin_permissions, plugin_setting_summaries, plugin_view_summaries, runtime_kind_label,
    },
    store::AppearanceStore,
    themes::built_in_theme_catalog,
    types::*,
};

impl AppearanceStore {
    pub(super) fn state(&self) -> Result<AppearanceState, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read appearance settings: {error}"))?;
        let settings = transaction
            .open_table(SETTINGS_KV)
            .map_err(|error| format!("failed to open appearance settings table: {error}"))?;
        let plugin_manifests = transaction
            .open_table(PLUGIN_MANIFESTS)
            .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
        let theme_manifests = transaction
            .open_table(THEME_MANIFESTS)
            .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
        let plugin_enablement = transaction
            .open_table(PLUGIN_ENABLEMENT)
            .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
        let plugin_settings = transaction
            .open_table(PLUGIN_SETTINGS)
            .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
        let plugin_installs = transaction
            .open_table(PLUGIN_INSTALLS)
            .map_err(|error| format!("failed to open plugin installs table: {error}"))?;

        let mut plugins = Vec::new();
        for item in plugin_manifests
            .iter()
            .map_err(|error| format!("failed to scan plugin manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read plugin manifest: {error}"))?;
            let manifest = decode_plugin_manifest(value.value())?;
            let enabled = plugin_enabled_from_table(&plugin_enablement, &manifest.id)?;
            let settings = plugin_setting_summaries(&plugin_settings, &manifest)?;
            let setting_count = settings.len();
            let capabilities = plugin_capability_summaries(&manifest);
            let actions = plugin_action_summaries(&manifest);
            let views = plugin_view_summaries(&manifest);
            let permissions = plugin_permissions(&manifest);
            let install = plugin_install_from_table(&plugin_installs, &manifest.id)?;
            plugins.push(ThemePluginSummary {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
                api_version: manifest.api_version,
                min_host_version: manifest.min_host_version,
                author: manifest.author,
                update_url: manifest.update_url,
                description: manifest.description,
                enabled,
                package_kind: install
                    .as_ref()
                    .map(|install| install.package_kind.clone())
                    .unwrap_or_else(|| "legacyManifest".to_string()),
                install_path: install.as_ref().map(|install| install.install_path.clone()),
                installed_at_ms: install.as_ref().map(|install| install.installed_at_ms),
                theme_count: manifest.contributes.themes.len(),
                runtime: runtime_kind_label(&manifest.runtime.kind).to_string(),
                events: manifest.runtime.events.clone(),
                capability_count: manifest.contributes.capabilities.len(),
                setting_count,
                action_count: manifest.contributes.actions.len(),
                permissions,
                capabilities,
                settings,
                actions,
                views,
            });
        }
        plugins.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));

        let mut themes = built_in_theme_catalog();
        for item in theme_manifests
            .iter()
            .map_err(|error| format!("failed to scan theme manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read theme manifest: {error}"))?;
            let stored = decode_stored_theme_manifest(value.value())?;
            let enabled = plugin_enabled_from_table(&plugin_enablement, &stored.plugin_id)?;
            themes.push(ThemeCatalogItem {
                id: stored.theme.id,
                name: stored.theme.name,
                version: stored.theme.version,
                source: "plugin".to_string(),
                plugin_id: Some(stored.plugin_id),
                enabled,
                tokens: stored.theme.tokens,
            });
        }
        themes.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });

        let requested_theme_id = settings
            .get(ACTIVE_THEME_KEY)
            .map_err(|error| format!("failed to read active theme setting: {error}"))?
            .map(|value| value.value().to_string())
            .unwrap_or_else(|| DEFAULT_THEME_ID.to_string());
        let active_theme_id = if themes
            .iter()
            .any(|theme| theme.id == requested_theme_id && theme.enabled)
        {
            requested_theme_id
        } else {
            DEFAULT_THEME_ID.to_string()
        };
        let accent_override = settings
            .get(ACCENT_OVERRIDE_KEY)
            .map_err(|error| format!("failed to read accent override setting: {error}"))?
            .map(|value| value.value().to_string());

        Ok(AppearanceState {
            active_theme_id,
            accent_override,
            themes,
            plugins,
        })
    }
}
