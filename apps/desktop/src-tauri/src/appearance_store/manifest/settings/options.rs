use std::collections::HashSet;

use super::super::primitives::{validate_localized_text_map, validate_non_empty};
use crate::appearance_store::types::PluginSettingManifest;

pub(super) fn validate_setting_number_bounds(
    setting: &PluginSettingManifest,
) -> Result<(), String> {
    if let Some(min) = setting.min
        && !min.is_finite()
    {
        return Err(format!("plugin setting {} min is invalid", setting.id));
    }
    if let Some(max) = setting.max
        && !max.is_finite()
    {
        return Err(format!("plugin setting {} max is invalid", setting.id));
    }
    if let Some(step) = setting.step
        && (!step.is_finite() || step <= 0.0)
    {
        return Err(format!("plugin setting {} step is invalid", setting.id));
    }
    if let (Some(min), Some(max)) = (setting.min, setting.max)
        && min > max
    {
        return Err(format!("plugin setting {} min exceeds max", setting.id));
    }
    Ok(())
}

pub(super) fn validate_setting_options(setting: &PluginSettingManifest) -> Result<(), String> {
    if setting.kind != "select" {
        if !setting.options.is_empty() {
            return Err(format!(
                "plugin setting {} options are only valid for select settings",
                setting.id
            ));
        }
        return Ok(());
    }
    if setting.options.is_empty() {
        return Err(format!(
            "plugin setting {} select options cannot be empty",
            setting.id
        ));
    }
    let mut values = HashSet::new();
    for option in &setting.options {
        validate_non_empty("plugin setting option value", &option.value)?;
        validate_non_empty("plugin setting option label", &option.label)?;
        validate_localized_text_map("plugin setting option labelI18n", &option.label_i18n, 128)?;
        if !values.insert(option.value.as_str()) {
            return Err(format!(
                "duplicate option value for plugin setting {}: {}",
                setting.id, option.value
            ));
        }
    }
    Ok(())
}
