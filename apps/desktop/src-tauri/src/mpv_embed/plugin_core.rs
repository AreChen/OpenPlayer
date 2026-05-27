use super::*;

const MAX_PLUGIN_MPV_TEXT_ARG_LEN: usize = 512;
const MAX_PLUGIN_MPV_SCRIPT_ARG_LEN: usize = 128;
const MAX_PLUGIN_MPV_SCRIPT_ARGS: usize = 8;
const MAX_PLUGIN_FILTER_LABEL_LEN: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PluginMpvCoreValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PluginMpvCorePropertyWrite {
    pub(super) property: &'static str,
    pub(super) value: PluginMpvCoreValue,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PluginMpvCoreCommand {
    pub(super) command: &'static str,
    pub(super) args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PluginMpvVideoFilter {
    pub(super) label: String,
    pub(super) expression: String,
}

enum PluginMpvCorePropertyReadKind {
    Bool,
    Number,
    Text,
}

pub(super) fn normalize_plugin_core_property(
    property: &str,
    value: &Value,
) -> Result<PluginMpvCorePropertyWrite, String> {
    let property = property.trim();
    let value = match property {
        "pause" | "mute" | "deinterlace" => PluginMpvCoreValue::Bool(plugin_bool_value(value)?),
        "volume" => PluginMpvCoreValue::Number(plugin_bounded_number(value, 0.0, 200.0, property)?),
        "speed" => PluginMpvCoreValue::Number(plugin_bounded_number(value, 0.25, 4.0, property)?),
        "sub-delay" | "audio-delay" => {
            PluginMpvCoreValue::Number(plugin_bounded_number(value, -60.0, 60.0, property)?)
        }
        "ab-loop-a" | "ab-loop-b" => {
            PluginMpvCoreValue::Number(plugin_bounded_number(value, 0.0, 86_400.0, property)?)
        }
        "brightness" | "contrast" | "saturation" | "gamma" | "hue" => {
            PluginMpvCoreValue::Number(plugin_bounded_number(value, -100.0, 100.0, property)?)
        }
        "panscan" => PluginMpvCoreValue::Number(plugin_bounded_number(value, 0.0, 1.0, property)?),
        "video-rotate" => {
            let rotation = plugin_number_value(value)?;
            if !matches!(rotation as i64, 0 | 90 | 180 | 270) || rotation.fract() != 0.0 {
                return Err("invalid plugin mpv video-rotate value".to_string());
            }
            PluginMpvCoreValue::Number(rotation)
        }
        "loop-file" => {
            let text = plugin_string_value(value)?;
            if !matches!(text.as_str(), "no" | "inf") {
                return Err("invalid plugin mpv loop-file value".to_string());
            }
            PluginMpvCoreValue::Text(text)
        }
        "aid" | "sid" | "vid" => normalize_plugin_track_property_value(value, property)?,
        other => return Err(format!("unsupported plugin mpv core property: {other}")),
    };

    Ok(PluginMpvCorePropertyWrite {
        property: plugin_core_property_name(property)?,
        value,
    })
}

pub(super) fn plugin_core_property_name(property: &str) -> Result<&'static str, String> {
    match property.trim() {
        "pause" => Ok("pause"),
        "mute" => Ok("mute"),
        "deinterlace" => Ok("deinterlace"),
        "volume" => Ok("volume"),
        "speed" => Ok("speed"),
        "sub-delay" => Ok("sub-delay"),
        "audio-delay" => Ok("audio-delay"),
        "ab-loop-a" => Ok("ab-loop-a"),
        "ab-loop-b" => Ok("ab-loop-b"),
        "brightness" => Ok("brightness"),
        "contrast" => Ok("contrast"),
        "saturation" => Ok("saturation"),
        "gamma" => Ok("gamma"),
        "hue" => Ok("hue"),
        "panscan" => Ok("panscan"),
        "video-rotate" => Ok("video-rotate"),
        "loop-file" => Ok("loop-file"),
        "aid" => Ok("aid"),
        "sid" => Ok("sid"),
        "vid" => Ok("vid"),
        other => Err(format!("unsupported plugin mpv core property: {other}")),
    }
}

pub(super) fn read_plugin_core_property(
    mpv: &libmpv2::Mpv,
    property: &'static str,
) -> Result<Value, String> {
    match plugin_core_property_read_kind(property) {
        PluginMpvCorePropertyReadKind::Bool => mpv
            .get_property::<bool>(property)
            .map(Value::Bool)
            .map_err(|error| format!("mpv plugin property read failed: {error}")),
        PluginMpvCorePropertyReadKind::Number => mpv
            .get_property::<f64>(property)
            .map(Value::from)
            .map_err(|error| format!("mpv plugin property read failed: {error}")),
        PluginMpvCorePropertyReadKind::Text => mpv
            .get_property::<String>(property)
            .map(Value::String)
            .map_err(|error| format!("mpv plugin property read failed: {error}")),
    }
}

pub(super) fn set_plugin_core_property_value(
    mpv: &libmpv2::Mpv,
    property: &str,
    value: &PluginMpvCoreValue,
) -> Result<(), String> {
    match value {
        PluginMpvCoreValue::Bool(value) => mpv
            .set_property(property, *value)
            .map_err(|error| error.to_string()),
        PluginMpvCoreValue::Number(value) => mpv
            .set_property(property, *value)
            .map_err(|error| error.to_string()),
        PluginMpvCoreValue::Text(value) => mpv
            .set_property(property, value.as_str())
            .map_err(|error| error.to_string()),
    }
}

pub(super) fn normalize_plugin_core_command(
    command: &str,
    args: &Value,
) -> Result<PluginMpvCoreCommand, String> {
    match command.trim() {
        "show-text" => normalize_show_text_command(args),
        "script-message" => normalize_script_message_command(args),
        "seek" => normalize_seek_command(args),
        "frame-step" => normalize_no_arg_command("frame-step", args),
        "frame-back-step" => normalize_no_arg_command("frame-back-step", args),
        "playlist-next" => normalize_no_arg_command("playlist-next", args),
        "playlist-prev" => normalize_no_arg_command("playlist-prev", args),
        "chapter-next" => normalize_no_arg_command("chapter-next", args),
        "chapter-prev" => normalize_no_arg_command("chapter-prev", args),
        "playlist-shuffle" => normalize_no_arg_command("playlist-shuffle", args),
        other => Err(format!("unsupported plugin mpv command: {other}")),
    }
}

pub(super) fn normalize_plugin_video_filter(
    plugin_id: &str,
    filter_id: &str,
    filter: &str,
    params: &Value,
) -> Result<PluginMpvVideoFilter, String> {
    let label = plugin_video_filter_label(plugin_id, filter_id)?;
    let filter = filter.trim();
    let expression = match filter {
        "eq" => {
            let options = normalize_eq_filter_options(params)?;
            if options.is_empty() {
                format!("@{label}:eq")
            } else {
                format!("@{label}:eq={}", options.join(":"))
            }
        }
        "hflip" | "vflip" => {
            reject_filter_params(params, filter)?;
            format!("@{label}:{filter}")
        }
        other => return Err(format!("unsupported plugin video filter: {other}")),
    };

    Ok(PluginMpvVideoFilter { label, expression })
}

pub(super) fn plugin_video_filter_remove_target(
    plugin_id: &str,
    filter_id: &str,
) -> Result<String, String> {
    Ok(format!(
        "@{}",
        plugin_video_filter_label(plugin_id, filter_id)?
    ))
}

pub(super) fn normalize_plugin_audio_filter(
    plugin_id: &str,
    filter_id: &str,
    filter: &str,
    params: &Value,
) -> Result<PluginMpvVideoFilter, String> {
    let label = plugin_video_filter_label(plugin_id, filter_id)?;
    let filter = filter.trim();
    let expression = match filter {
        "volume" => {
            let options = normalize_volume_filter_options(params)?;
            format!("@{label}:volume={options}")
        }
        other => return Err(format!("unsupported plugin audio filter: {other}")),
    };

    Ok(PluginMpvVideoFilter { label, expression })
}

pub(super) fn plugin_audio_filter_remove_target(
    plugin_id: &str,
    filter_id: &str,
) -> Result<String, String> {
    Ok(format!(
        "@{}",
        plugin_video_filter_label(plugin_id, filter_id)?
    ))
}

fn plugin_core_property_read_kind(property: &str) -> PluginMpvCorePropertyReadKind {
    match property {
        "pause" | "mute" | "deinterlace" => PluginMpvCorePropertyReadKind::Bool,
        "loop-file" | "aid" | "sid" | "vid" => PluginMpvCorePropertyReadKind::Text,
        _ => PluginMpvCorePropertyReadKind::Number,
    }
}

fn normalize_plugin_track_property_value(
    value: &Value,
    property: &str,
) -> Result<PluginMpvCoreValue, String> {
    if let Some(text) = value.as_str() {
        if matches!(text, "no" | "auto") {
            return Ok(PluginMpvCoreValue::Text(text.to_string()));
        }
        return Err(format!("invalid plugin mpv {property} value"));
    }

    let track_id = plugin_number_value(value)?;
    if track_id.fract() != 0.0 || !(0.0..=100_000.0).contains(&track_id) {
        return Err(format!("invalid plugin mpv {property} value"));
    }
    Ok(PluginMpvCoreValue::Number(track_id))
}

fn normalize_show_text_command(args: &Value) -> Result<PluginMpvCoreCommand, String> {
    let args = plugin_args_array(args)?;
    if !(1..=2).contains(&args.len()) {
        return Err("plugin mpv show-text expects text and optional duration".to_string());
    }
    let text = plugin_string_arg(&args[0], MAX_PLUGIN_MPV_TEXT_ARG_LEN, "show-text text")?;
    let duration = if let Some(duration) = args.get(1) {
        plugin_integer_arg(duration, 0, 10_000, "show-text duration")?
    } else {
        2_000
    };
    Ok(PluginMpvCoreCommand {
        command: "show-text",
        args: vec![text, duration.to_string()],
    })
}

fn normalize_script_message_command(args: &Value) -> Result<PluginMpvCoreCommand, String> {
    let args = plugin_args_array(args)?;
    if args.is_empty() || args.len() > MAX_PLUGIN_MPV_SCRIPT_ARGS {
        return Err("plugin mpv script-message expects 1 to 8 string args".to_string());
    }
    let args = args
        .iter()
        .map(|arg| {
            plugin_string_arg(
                arg,
                MAX_PLUGIN_MPV_SCRIPT_ARG_LEN,
                "script-message argument",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PluginMpvCoreCommand {
        command: "script-message",
        args,
    })
}

fn normalize_seek_command(args: &Value) -> Result<PluginMpvCoreCommand, String> {
    let args = plugin_args_array(args)?;
    if args.is_empty() || args.len() > 3 {
        return Err(
            "plugin mpv seek expects target, optional mode, and optional precision".to_string(),
        );
    }
    let target = plugin_bounded_number(&args[0], -86_400.0, 86_400.0, "seek target")?;
    let mode = args
        .get(1)
        .map(|value| plugin_string_arg(value, 32, "seek mode"))
        .transpose()?
        .unwrap_or_else(|| "relative".to_string());
    if !matches!(
        mode.as_str(),
        "relative" | "absolute" | "absolute-percent" | "relative-percent"
    ) {
        return Err("invalid plugin mpv seek mode".to_string());
    }
    let precision = args
        .get(2)
        .map(|value| plugin_string_arg(value, 32, "seek precision"))
        .transpose()?;
    if let Some(precision) = precision.as_deref()
        && !matches!(precision, "keyframes" | "exact")
    {
        return Err("invalid plugin mpv seek precision".to_string());
    }

    let mut normalized_args = vec![format_plugin_number(target), mode];
    if let Some(precision) = precision {
        normalized_args.push(precision);
    }
    Ok(PluginMpvCoreCommand {
        command: "seek",
        args: normalized_args,
    })
}

fn normalize_no_arg_command(
    command: &'static str,
    args: &Value,
) -> Result<PluginMpvCoreCommand, String> {
    let args = plugin_args_array(args)?;
    if !args.is_empty() {
        return Err(format!("plugin mpv {command} does not accept args"));
    }
    Ok(PluginMpvCoreCommand {
        command,
        args: Vec::new(),
    })
}

fn normalize_eq_filter_options(params: &Value) -> Result<Vec<String>, String> {
    let object = plugin_params_object(params, "eq")?;
    let mut options = Vec::new();
    for key in ["brightness", "contrast", "saturation", "gamma", "hue"] {
        if let Some(value) = object.get(key) {
            let value = plugin_bounded_number(value, -100.0, 100.0, key)?;
            options.push(format!("{key}={}", format_plugin_number(value)));
        }
    }
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "brightness" | "contrast" | "saturation" | "gamma" | "hue"
        ) {
            return Err(format!("unsupported plugin eq filter option: {key}"));
        }
    }
    Ok(options)
}

fn normalize_volume_filter_options(params: &Value) -> Result<String, String> {
    let object = plugin_params_object(params, "volume")?;
    let gain_db = object
        .get("gainDb")
        .map(|value| plugin_bounded_number(value, -24.0, 24.0, "volume gainDb"))
        .transpose()?
        .unwrap_or(0.0);
    for key in object.keys() {
        if key != "gainDb" {
            return Err(format!("unsupported plugin volume filter option: {key}"));
        }
    }
    Ok(format!("volume={}dB", format_plugin_number(gain_db)))
}

fn reject_filter_params(params: &Value, filter: &str) -> Result<(), String> {
    let object = plugin_params_object(params, filter)?;
    if object.is_empty() {
        Ok(())
    } else {
        Err(format!("plugin {filter} filter does not accept options"))
    }
}

fn plugin_video_filter_label(plugin_id: &str, filter_id: &str) -> Result<String, String> {
    validate_plugin_identifier_part(plugin_id, "plugin id")?;
    validate_plugin_identifier_part(filter_id, "filter id")?;
    let label = format!(
        "op_{}_{}",
        sanitize_filter_label_part(plugin_id),
        sanitize_filter_label_part(filter_id)
    );
    if label.len() > MAX_PLUGIN_FILTER_LABEL_LEN {
        return Err("plugin video filter label is too long".to_string());
    }
    Ok(label)
}

fn validate_plugin_identifier_part(value: &str, label: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | '-'))
    {
        return Err(format!("invalid plugin video filter {label}"));
    }
    Ok(())
}

fn sanitize_filter_label_part(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn plugin_bool_value(value: &Value) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| "plugin mpv property expects a boolean".to_string())
}

fn plugin_bounded_number(value: &Value, min: f64, max: f64, label: &str) -> Result<f64, String> {
    let value = plugin_number_value(value)?;
    if !(min..=max).contains(&value) {
        return Err(format!("plugin mpv {label} value is out of range"));
    }
    Ok(value)
}

fn plugin_args_array(value: &Value) -> Result<&Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| "plugin mpv command args must be an array".to_string())
}

fn plugin_params_object<'a>(
    value: &'a Value,
    filter: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    if value.is_null() {
        static EMPTY: std::sync::OnceLock<serde_json::Map<String, Value>> =
            std::sync::OnceLock::new();
        return Ok(EMPTY.get_or_init(serde_json::Map::new));
    }
    value
        .as_object()
        .ok_or_else(|| format!("plugin {filter} filter params must be an object"))
}

fn plugin_string_arg(value: &Value, max_len: usize, label: &str) -> Result<String, String> {
    let text = plugin_string_value(value)?;
    if text.is_empty() || text.len() > max_len {
        return Err(format!("invalid plugin mpv {label}"));
    }
    Ok(text)
}

fn plugin_integer_arg(value: &Value, min: i64, max: i64, label: &str) -> Result<i64, String> {
    let value = plugin_number_value(value)?;
    if value.fract() != 0.0 || value < min as f64 || value > max as f64 {
        return Err(format!("invalid plugin mpv {label}"));
    }
    Ok(value as i64)
}
