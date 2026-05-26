use serde_json::Value;

use super::super::primitives::validate_color_token;
use crate::appearance_store::types::PluginSettingManifest;

pub(in crate::appearance_store) fn validate_plugin_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    match setting.kind.as_str() {
        "boolean" => validate_boolean_setting_value(setting, value),
        "number" => validate_number_setting_value(setting, value),
        "text" | "directory" => validate_text_setting_value(setting, value),
        "select" => validate_select_setting_value(setting, value),
        "color" => validate_color_setting_value(setting, value),
        _ => Err(format!("unsupported plugin setting kind: {}", setting.kind)),
    }
}

fn validate_boolean_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    if value.as_bool().is_some() {
        Ok(())
    } else {
        Err(format!("plugin setting {} expects a boolean", setting.id))
    }
}

fn validate_number_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    let Some(number) = value.as_f64().filter(|value| value.is_finite()) else {
        return Err(format!("plugin setting {} expects a number", setting.id));
    };
    if let Some(min) = setting.min
        && number < min
    {
        return Err(format!("plugin setting {} is below minimum", setting.id));
    }
    if let Some(max) = setting.max
        && number > max
    {
        return Err(format!("plugin setting {} is above maximum", setting.id));
    }
    Ok(())
}

fn validate_text_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    let Some(text) = value.as_str() else {
        return Err(format!("plugin setting {} expects text", setting.id));
    };
    if text.len() > 1024 {
        return Err(format!("plugin setting {} text is too long", setting.id));
    }
    Ok(())
}

fn validate_select_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    let Some(selected) = value.as_str() else {
        return Err(format!("plugin setting {} expects a selection", setting.id));
    };
    if setting
        .options
        .iter()
        .any(|option| option.value == selected)
    {
        Ok(())
    } else {
        Err(format!(
            "plugin setting {} has an unknown option",
            setting.id
        ))
    }
}

fn validate_color_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    let Some(color) = value.as_str() else {
        return Err(format!("plugin setting {} expects a color", setting.id));
    };
    validate_color_token(&setting.id, color)
}
