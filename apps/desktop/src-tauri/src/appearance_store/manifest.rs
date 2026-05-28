use std::collections::HashSet;

mod actions;
mod primitives;
mod runtime;
mod settings;
mod storage;
mod theme;

use actions::validate_plugin_action;
use primitives::{
    compare_simple_semver, validate_http_url, validate_non_empty, validate_simple_semver,
};
use runtime::{validate_plugin_capability, validate_plugin_runtime, validate_plugin_view};
use settings::validate_plugin_setting;
use storage::validate_plugin_storage;
use theme::validate_theme_manifest;

use super::{SUPPORTED_PLUGIN_API_VERSION, records::plugin_permissions, types::*};

pub(super) fn parse_theme_plugin_manifest_json(json: &str) -> Result<PluginManifest, String> {
    let manifest: PluginManifest = serde_json::from_str(json)
        .map_err(|error| format!("invalid plugin manifest JSON: {error}"))?;
    validate_plugin_manifest(&manifest)?;
    Ok(manifest)
}

pub(super) fn validate_color_token(token: &str, value: &str) -> Result<(), String> {
    primitives::validate_color_token(token, value)
}

pub(super) fn validate_dotted_identifier(
    label: &str,
    value: &str,
    require_dot: bool,
) -> Result<(), String> {
    primitives::validate_dotted_identifier(label, value, require_dot)
}

pub(super) fn validate_relative_plugin_entry(entry: &str) -> Result<(), String> {
    runtime::validate_relative_plugin_entry(entry)
}

#[cfg(test)]
pub(super) fn is_supported_plugin_permission(permission: &str) -> bool {
    runtime::is_supported_plugin_permission(permission)
}

pub(super) fn validate_plugin_setting_value(
    setting: &PluginSettingManifest,
    value: &serde_json::Value,
) -> Result<(), String> {
    settings::validate_plugin_setting_value(setting, value)
}

pub(super) fn is_allowed_plugin_mpv_property(property: &str) -> bool {
    settings::is_allowed_plugin_mpv_property(property)
}

pub(super) fn validate_plugin_manifest(manifest: &PluginManifest) -> Result<(), String> {
    validate_non_empty("plugin id", &manifest.id)?;
    validate_non_empty("plugin name", &manifest.name)?;
    validate_non_empty("plugin version", &manifest.version)?;
    validate_dotted_identifier("plugin id", &manifest.id, true)?;
    validate_simple_semver("plugin version", &manifest.version)?;
    validate_non_empty("plugin apiVersion", &manifest.api_version)?;
    if manifest.api_version != SUPPORTED_PLUGIN_API_VERSION {
        return Err(format!(
            "unsupported plugin apiVersion: {}",
            manifest.api_version
        ));
    }
    if let Some(min_host_version) = manifest.min_host_version.as_deref() {
        validate_simple_semver("plugin minHostVersion", min_host_version)?;
        if compare_simple_semver(min_host_version, env!("CARGO_PKG_VERSION"))?.is_gt() {
            return Err(format!(
                "plugin {} requires OpenPlayer {min_host_version} or newer",
                manifest.id
            ));
        }
    }
    if let Some(author) = manifest.author.as_deref() {
        validate_non_empty("plugin author", author)?;
        if author.len() > 128 {
            return Err("plugin author is too long".to_string());
        }
    }
    if let Some(update_url) = manifest.update_url.as_deref() {
        validate_http_url("plugin updateUrl", update_url)?;
    }
    if let Some(description) = manifest.description.as_deref() {
        validate_non_empty("plugin description", description)?;
    }
    validate_plugin_runtime(&manifest.runtime)?;
    if manifest.contributes.themes.is_empty()
        && manifest.contributes.capabilities.is_empty()
        && manifest.contributes.settings.is_empty()
        && manifest.contributes.actions.is_empty()
        && manifest.contributes.views.is_empty()
        && manifest.contributes.storage.is_none()
    {
        return Err(
            "plugin must contribute at least one theme, capability, setting, action, view, or storage schema"
                .to_string(),
        );
    }

    if let Some(storage) = manifest.contributes.storage.as_ref() {
        validate_plugin_storage(storage)?;
    }

    let mut ids = HashSet::new();
    for theme in &manifest.contributes.themes {
        validate_theme_manifest(theme)?;
        if !ids.insert(theme.id.as_str()) {
            return Err(format!("duplicate theme id: {}", theme.id));
        }
    }

    let mut capability_ids = HashSet::new();
    for capability in &manifest.contributes.capabilities {
        validate_plugin_capability(capability)?;
        if !capability_ids.insert(capability.id.as_str()) {
            return Err(format!("duplicate capability id: {}", capability.id));
        }
    }

    let mut setting_ids = HashSet::new();
    for setting in &manifest.contributes.settings {
        validate_plugin_setting(setting)?;
        if !setting_ids.insert(setting.id.as_str()) {
            return Err(format!("duplicate setting id: {}", setting.id));
        }
    }

    let permissions = plugin_permissions(manifest);
    let mut action_ids = HashSet::new();
    for action in &manifest.contributes.actions {
        validate_plugin_action(action, &permissions)?;
        if !action_ids.insert(action.id.as_str()) {
            return Err(format!("duplicate action id: {}", action.id));
        }
    }

    let mut view_ids = HashSet::new();
    for view in &manifest.contributes.views {
        validate_plugin_view(view)?;
        if let Some(setting_id) = view.frame_opacity_setting.as_deref() {
            let Some(setting) = manifest
                .contributes
                .settings
                .iter()
                .find(|candidate| candidate.id == setting_id)
            else {
                return Err(format!(
                    "plugin view {} frameOpacitySetting references unknown setting: {}",
                    view.id, setting_id
                ));
            };
            if setting.kind != "number" {
                return Err(format!(
                    "plugin view {} frameOpacitySetting must reference a number setting",
                    view.id
                ));
            }
        }
        if !view_ids.insert(view.id.as_str()) {
            return Err(format!("duplicate view id: {}", view.id));
        }
    }

    Ok(())
}
