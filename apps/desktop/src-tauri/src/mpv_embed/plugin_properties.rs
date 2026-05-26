use super::*;

#[derive(Debug, PartialEq)]
pub(super) enum PluginMpvPropertyValue {
    Text(String),
    Number(f64),
}

pub(super) fn normalize_plugin_mpv_property(
    property: &str,
    value: &Value,
) -> Result<(&'static str, PluginMpvPropertyValue), String> {
    match property.trim() {
        "sub-font" => {
            let text = plugin_string_value(value)?;
            if text.trim().is_empty() || text.len() > 128 {
                return Err("invalid plugin subtitle font".to_string());
            }
            Ok(("sub-font", PluginMpvPropertyValue::Text(text)))
        }
        "sub-font-size" => {
            let size = plugin_number_value(value)?;
            if !(1.0..=128.0).contains(&size) {
                return Err("invalid plugin subtitle font size".to_string());
            }
            Ok(("sub-font-size", PluginMpvPropertyValue::Number(size)))
        }
        "sub-scale" => {
            let scale = plugin_number_value(value)?;
            if !(0.1..=5.0).contains(&scale) {
                return Err("invalid plugin subtitle scale".to_string());
            }
            Ok(("sub-scale", PluginMpvPropertyValue::Number(scale)))
        }
        "sub-pos" => {
            let position = plugin_number_value(value)?;
            if !(0.0..=100.0).contains(&position) {
                return Err("invalid plugin subtitle position".to_string());
            }
            Ok(("sub-pos", PluginMpvPropertyValue::Number(position)))
        }
        "sub-color" => {
            let color = plugin_string_value(value)?;
            if !is_plugin_hex_color(&color) {
                return Err("invalid plugin subtitle color".to_string());
            }
            Ok(("sub-color", PluginMpvPropertyValue::Text(color)))
        }
        "sub-spacing" => {
            let spacing = plugin_number_value(value)?;
            if !(-10.0..=10.0).contains(&spacing) {
                return Err("invalid plugin subtitle spacing".to_string());
            }
            Ok((
                "sub-spacing",
                PluginMpvPropertyValue::Text(format_plugin_number(spacing)),
            ))
        }
        "sub-outline-size" | "sub-border-size" => {
            let outline_size = plugin_number_value(value)?;
            if !(0.0..=32.0).contains(&outline_size) {
                return Err("invalid plugin subtitle outline size".to_string());
            }
            Ok((
                "sub-outline-size",
                PluginMpvPropertyValue::Number(outline_size),
            ))
        }
        "sub-shadow-offset" => {
            let shadow_offset = plugin_number_value(value)?;
            if !(0.0..=32.0).contains(&shadow_offset) {
                return Err("invalid plugin subtitle shadow offset".to_string());
            }
            Ok((
                "sub-shadow-offset",
                PluginMpvPropertyValue::Number(shadow_offset),
            ))
        }
        other => Err(format!("unsupported plugin mpv property: {other}")),
    }
}

pub(super) fn plugin_string_value(value: &Value) -> Result<String, String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "plugin mpv property expects text".to_string())
}

pub(super) fn plugin_number_value(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| "plugin mpv property expects a number".to_string())
}

pub(super) fn format_plugin_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

pub(super) fn set_plugin_mpv_property_value(
    mpv: &libmpv2::Mpv,
    property: &str,
    value: &PluginMpvPropertyValue,
) -> Result<(), String> {
    match value {
        PluginMpvPropertyValue::Text(value) => mpv
            .set_property(property, value.as_str())
            .map_err(|error| error.to_string()),
        PluginMpvPropertyValue::Number(value) => mpv
            .set_property(property, *value)
            .map_err(|error| error.to_string()),
    }
}

pub(super) fn plugin_mpv_property_write_targets(property: &'static str) -> &'static [&'static str] {
    match property {
        "sub-font" => &["sub-font"],
        "sub-font-size" => &["sub-font-size"],
        "sub-scale" => &["sub-scale"],
        "sub-pos" => &["sub-pos"],
        "sub-color" => &["sub-color"],
        "sub-spacing" => &["sub-spacing"],
        "sub-outline-size" => &["sub-outline-size"],
        "sub-shadow-offset" => &["sub-shadow-offset"],
        _ => &[],
    }
}

pub(super) fn plugin_subtitle_style_requires_ass_override(property: &str) -> bool {
    matches!(
        property,
        "sub-font"
            | "sub-font-size"
            | "sub-scale"
            | "sub-pos"
            | "sub-color"
            | "sub-spacing"
            | "sub-outline-size"
            | "sub-shadow-offset"
    )
}

pub(super) fn is_plugin_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|char| char.is_ascii_hexdigit())
}
