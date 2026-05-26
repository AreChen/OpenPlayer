use serde_json::Value;

use super::catalog::is_plugin_runtime_action_command;
use super::stream::validate_plugin_stream_url;
use crate::appearance_store::types::PluginActionManifest;

pub(super) fn validate_plugin_action_args(action: &PluginActionManifest) -> Result<(), String> {
    let Some(args) = action.args.as_object() else {
        return Err(format!(
            "plugin action {} args must be an object",
            action.id
        ));
    };

    if is_plugin_runtime_action_command(&action.command) {
        let serialized = serde_json::to_string(args)
            .map_err(|error| format!("plugin action {} args are invalid: {error}", action.id))?;
        if serialized.len() > 4096 {
            return Err(format!("plugin action {} args are too large", action.id));
        }
        return Ok(());
    }

    match action.command.as_str() {
        "player.captureScreenshot" => {
            validate_capture_recording_action_args(action, args)?;
            Ok(())
        }
        "player.startRecording" | "player.toggleRecording" => {
            validate_capture_recording_action_args(action, args)?;
            Ok(())
        }
        "player.stopRecording" => validate_stop_recording_action_args(action, args),
        "player.openStream" => validate_open_stream_action_args(action, args),
        _ => validate_empty_action_args(action, args),
    }
}

fn validate_capture_recording_action_args(
    action: &PluginActionManifest,
    args: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    for key in args.keys() {
        if !matches!(
            key.as_str(),
            "openFolder" | "openFolderSetting" | "format" | "formatSetting" | "directorySetting"
        ) {
            return Err(format!(
                "plugin action {} has unknown argument: {key}",
                action.id
            ));
        }
    }
    validate_optional_plugin_action_boolean_arg(action, args, "openFolder")?;
    validate_optional_plugin_action_string_arg(action, args, "format", 16)?;
    validate_optional_plugin_action_string_arg(action, args, "formatSetting", 96)?;
    validate_optional_plugin_action_string_arg(action, args, "openFolderSetting", 96)?;
    validate_optional_plugin_action_string_arg(action, args, "directorySetting", 96)?;
    Ok(())
}

fn validate_stop_recording_action_args(
    action: &PluginActionManifest,
    args: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    for key in args.keys() {
        if !matches!(key.as_str(), "openFolder" | "openFolderSetting") {
            return Err(format!(
                "plugin action {} has unknown argument: {key}",
                action.id
            ));
        }
    }
    validate_optional_plugin_action_boolean_arg(action, args, "openFolder")?;
    validate_optional_plugin_action_string_arg(action, args, "openFolderSetting", 96)?;
    Ok(())
}

fn validate_open_stream_action_args(
    action: &PluginActionManifest,
    args: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    let Some(url) = args.get("url").and_then(Value::as_str) else {
        return Err(format!("plugin action {} requires a stream url", action.id));
    };
    validate_plugin_stream_url(url)?;
    for key in args.keys() {
        if key != "url" && key != "name" {
            return Err(format!(
                "plugin action {} has unknown argument: {key}",
                action.id
            ));
        }
    }
    if let Some(name) = args.get("name").and_then(Value::as_str)
        && (name.trim().is_empty() || name.len() > 128)
    {
        return Err(format!(
            "plugin action {} stream name is invalid",
            action.id
        ));
    }
    Ok(())
}

fn validate_empty_action_args(
    action: &PluginActionManifest,
    args: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "plugin action {} does not accept arguments",
            action.id
        ))
    }
}

fn validate_optional_plugin_action_boolean_arg(
    action: &PluginActionManifest,
    args: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<(), String> {
    if let Some(value) = args.get(key)
        && !value.is_boolean()
    {
        return Err(format!(
            "plugin action {} {key} argument must be boolean",
            action.id
        ));
    }
    Ok(())
}

fn validate_optional_plugin_action_string_arg(
    action: &PluginActionManifest,
    args: &serde_json::Map<String, Value>,
    key: &str,
    max_len: usize,
) -> Result<(), String> {
    if let Some(value) = args.get(key) {
        let Some(value) = value.as_str() else {
            return Err(format!(
                "plugin action {} {key} argument must be text",
                action.id
            ));
        };
        if value.trim().is_empty() || value.len() > max_len {
            return Err(format!(
                "plugin action {} {key} argument is invalid",
                action.id
            ));
        }
    }
    Ok(())
}
