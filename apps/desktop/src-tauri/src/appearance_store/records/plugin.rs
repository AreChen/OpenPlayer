use redb::ReadableTable;

use crate::appearance_store::{
    manifest::{is_allowed_plugin_mpv_property, validate_plugin_setting_value},
    types::*,
};

use super::{
    codecs::{decode_plugin_install_record, decode_plugin_setting_value},
    storage::plugin_setting_key,
};

pub(in crate::appearance_store) fn plugin_enabled_from_table<T>(
    table: &T,
    plugin_id: &str,
) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    Ok(table
        .get(plugin_id)
        .map_err(|error| format!("failed to read plugin enablement: {error}"))?
        .map(|value| value.value() != "false")
        .unwrap_or(true))
}

pub(in crate::appearance_store) fn plugin_setting_summaries<T>(
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

pub(in crate::appearance_store) fn plugin_capability_summaries(
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

pub(in crate::appearance_store) fn plugin_action_summaries(
    manifest: &PluginManifest,
) -> Vec<PluginActionSummary> {
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

pub(in crate::appearance_store) fn plugin_view_summaries(
    manifest: &PluginManifest,
) -> Vec<PluginViewSummary> {
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

pub(in crate::appearance_store) fn plugin_permissions(manifest: &PluginManifest) -> Vec<String> {
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

pub(in crate::appearance_store) fn plugin_install_from_table<T>(
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
