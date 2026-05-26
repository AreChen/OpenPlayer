use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::{
    SUPPORTED_PLUGIN_API_VERSION,
    records::{plugin_permissions, runtime_kind_label},
    types::*,
};
pub(super) fn parse_theme_plugin_manifest_json(json: &str) -> Result<PluginManifest, String> {
    let manifest: PluginManifest = serde_json::from_str(json)
        .map_err(|error| format!("invalid plugin manifest JSON: {error}"))?;
    validate_plugin_manifest(&manifest)?;
    Ok(manifest)
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
    {
        return Err(
            "plugin must contribute at least one theme, capability, setting, action, or view"
                .to_string(),
        );
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
        if !view_ids.insert(view.id.as_str()) {
            return Err(format!("duplicate view id: {}", view.id));
        }
    }

    Ok(())
}

pub(super) fn validate_plugin_runtime(runtime: &PluginRuntime) -> Result<(), String> {
    match runtime.kind {
        PluginRuntimeKind::Manifest => {
            if let Some(entry) = runtime.entry.as_deref() {
                validate_relative_plugin_entry(entry)?;
            }
            if let Some(sandbox) = runtime.sandbox.as_deref() {
                validate_non_empty("plugin runtime sandbox", sandbox)?;
            }
            Ok(())
        }
        PluginRuntimeKind::WebviewJs => {
            let Some(entry) = runtime.entry.as_deref() else {
                return Err("plugin runtime webviewJs requires an entry".to_string());
            };
            validate_relative_plugin_entry(entry)?;
            if let Some(sandbox) = runtime.sandbox.as_deref()
                && sandbox != "openplayer-worker"
            {
                return Err(
                    "plugin runtime webviewJs requires the openplayer-worker sandbox".to_string(),
                );
            }
            Ok(())
        }
        PluginRuntimeKind::Wasm => Err(format!(
            "plugin runtime {} is not supported yet",
            runtime_kind_label(&runtime.kind)
        )),
    }
}

pub(super) fn validate_relative_plugin_entry(entry: &str) -> Result<(), String> {
    validate_non_empty("plugin runtime entry", entry)?;
    if entry.contains('\\') || entry.starts_with('/') || entry.contains("..") {
        Err("plugin runtime entry must be a relative package path".to_string())
    } else {
        Ok(())
    }
}

pub(super) fn validate_plugin_capability(
    capability: &PluginCapabilityManifest,
) -> Result<(), String> {
    validate_non_empty("plugin capability id", &capability.id)?;
    validate_non_empty("plugin capability name", &capability.name)?;
    validate_dotted_identifier("plugin capability id", &capability.id, false)?;
    if let Some(description) = capability.description.as_deref() {
        validate_non_empty("plugin capability description", description)?;
    }
    validate_localized_text_map("plugin capability nameI18n", &capability.name_i18n, 128)?;
    validate_localized_text_map(
        "plugin capability descriptionI18n",
        &capability.description_i18n,
        512,
    )?;
    if !is_supported_capability_kind(&capability.kind) {
        return Err(format!(
            "unsupported plugin capability kind: {}",
            capability.kind
        ));
    }
    for permission in &capability.permissions {
        if !is_supported_plugin_permission(permission) {
            return Err(format!("unsupported plugin permission: {permission}"));
        }
    }
    Ok(())
}

pub(super) fn validate_plugin_setting(setting: &PluginSettingManifest) -> Result<(), String> {
    validate_non_empty("plugin setting id", &setting.id)?;
    validate_non_empty("plugin setting label", &setting.label)?;
    validate_dotted_identifier("plugin setting id", &setting.id, false)?;
    if let Some(description) = setting.description.as_deref() {
        validate_non_empty("plugin setting description", description)?;
    }
    validate_localized_text_map("plugin setting labelI18n", &setting.label_i18n, 128)?;
    validate_localized_text_map(
        "plugin setting descriptionI18n",
        &setting.description_i18n,
        512,
    )?;
    if !is_supported_setting_kind(&setting.kind) {
        return Err(format!("unsupported plugin setting kind: {}", setting.kind));
    }
    if !is_supported_setting_placement(&setting.placement) {
        return Err(format!(
            "unsupported plugin setting placement: {}",
            setting.placement
        ));
    }
    validate_setting_number_bounds(setting)?;
    validate_setting_options(setting)?;
    if let Some(property) = setting.mpv_property.as_deref() {
        validate_plugin_mpv_property(property)?;
        if setting.placement != "subtitleSettings" {
            return Err(format!(
                "mpv property setting {} must use subtitleSettings placement",
                setting.id
            ));
        }
    }
    validate_plugin_setting_value(setting, &setting.default_value)
}

pub(super) fn validate_plugin_action(
    action: &PluginActionManifest,
    permissions: &[String],
) -> Result<(), String> {
    validate_non_empty("plugin action id", &action.id)?;
    validate_non_empty("plugin action label", &action.label)?;
    validate_dotted_identifier("plugin action id", &action.id, false)?;
    if let Some(description) = action.description.as_deref() {
        validate_non_empty("plugin action description", description)?;
    }
    validate_localized_text_map("plugin action labelI18n", &action.label_i18n, 128)?;
    validate_localized_text_map(
        "plugin action descriptionI18n",
        &action.description_i18n,
        512,
    )?;
    if !is_supported_action_placement(&action.placement) {
        return Err(format!(
            "unsupported plugin action placement: {}",
            action.placement
        ));
    }
    if !is_supported_plugin_action_command(&action.command) {
        return Err(format!(
            "unsupported plugin action command: {}",
            action.command
        ));
    }
    if let Some(icon) = action.icon.as_deref()
        && !is_supported_plugin_action_icon(icon)
    {
        return Err(format!("unsupported plugin action icon: {icon}"));
    }
    if let Some(permission) = plugin_action_required_permission(&action.command)
        && !permissions.iter().any(|item| item == permission)
    {
        return Err(format!(
            "plugin action {} requires permission {}",
            action.id, permission
        ));
    }
    validate_plugin_action_args(action)?;
    Ok(())
}

pub(super) fn validate_plugin_view(view: &PluginViewManifest) -> Result<(), String> {
    validate_non_empty("plugin view id", &view.id)?;
    validate_non_empty("plugin view title", &view.title)?;
    validate_dotted_identifier("plugin view id", &view.id, false)?;
    validate_relative_plugin_entry(&view.entry)?;
    if let Some(description) = view.description.as_deref() {
        validate_non_empty("plugin view description", description)?;
    }
    validate_localized_text_map("plugin view titleI18n", &view.title_i18n, 128)?;
    validate_localized_text_map("plugin view descriptionI18n", &view.description_i18n, 512)?;
    Ok(())
}

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

pub(super) fn validate_plugin_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    match setting.kind.as_str() {
        "boolean" => {
            if value.as_bool().is_some() {
                Ok(())
            } else {
                Err(format!("plugin setting {} expects a boolean", setting.id))
            }
        }
        "number" => {
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
        "text" | "directory" => {
            let Some(text) = value.as_str() else {
                return Err(format!("plugin setting {} expects text", setting.id));
            };
            if text.len() > 1024 {
                return Err(format!("plugin setting {} text is too long", setting.id));
            }
            Ok(())
        }
        "select" => {
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
        "color" => {
            let Some(color) = value.as_str() else {
                return Err(format!("plugin setting {} expects a color", setting.id));
            };
            validate_color_token(&setting.id, color)
        }
        _ => Err(format!("unsupported plugin setting kind: {}", setting.kind)),
    }
}

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
            for key in args.keys() {
                if !matches!(
                    key.as_str(),
                    "openFolder"
                        | "openFolderSetting"
                        | "format"
                        | "formatSetting"
                        | "directorySetting"
                ) {
                    return Err(format!(
                        "plugin action {} has unknown argument: {key}",
                        action.id
                    ));
                }
            }
            if let Some(open_folder) = args.get("openFolder")
                && !open_folder.is_boolean()
            {
                return Err(format!(
                    "plugin action {} openFolder argument must be boolean",
                    action.id
                ));
            }
            validate_optional_plugin_action_string_arg(action, args, "format", 16)?;
            validate_optional_plugin_action_string_arg(action, args, "formatSetting", 96)?;
            validate_optional_plugin_action_string_arg(action, args, "openFolderSetting", 96)?;
            validate_optional_plugin_action_string_arg(action, args, "directorySetting", 96)?;
            Ok(())
        }
        "player.startRecording" | "player.toggleRecording" => {
            for key in args.keys() {
                if !matches!(
                    key.as_str(),
                    "openFolder"
                        | "openFolderSetting"
                        | "format"
                        | "formatSetting"
                        | "directorySetting"
                ) {
                    return Err(format!(
                        "plugin action {} has unknown argument: {key}",
                        action.id
                    ));
                }
            }
            if let Some(open_folder) = args.get("openFolder")
                && !open_folder.is_boolean()
            {
                return Err(format!(
                    "plugin action {} openFolder argument must be boolean",
                    action.id
                ));
            }
            validate_optional_plugin_action_string_arg(action, args, "format", 16)?;
            validate_optional_plugin_action_string_arg(action, args, "formatSetting", 96)?;
            validate_optional_plugin_action_string_arg(action, args, "openFolderSetting", 96)?;
            validate_optional_plugin_action_string_arg(action, args, "directorySetting", 96)?;
            Ok(())
        }
        "player.stopRecording" => {
            for key in args.keys() {
                if !matches!(key.as_str(), "openFolder" | "openFolderSetting") {
                    return Err(format!(
                        "plugin action {} has unknown argument: {key}",
                        action.id
                    ));
                }
            }
            if let Some(open_folder) = args.get("openFolder")
                && !open_folder.is_boolean()
            {
                return Err(format!(
                    "plugin action {} openFolder argument must be boolean",
                    action.id
                ));
            }
            validate_optional_plugin_action_string_arg(action, args, "openFolderSetting", 96)?;
            Ok(())
        }
        "player.openStream" => {
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
        _ => {
            if args.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "plugin action {} does not accept arguments",
                    action.id
                ))
            }
        }
    }
}

pub(super) fn validate_optional_plugin_action_string_arg(
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

pub(super) fn validate_plugin_stream_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    validate_non_empty("plugin stream url", trimmed)?;
    if trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err("plugin stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err("plugin stream url must include a protocol".to_string());
    };
    if rest.trim_matches('/').is_empty() {
        return Err("plugin stream url must include a host or path".to_string());
    }
    if is_supported_plugin_stream_scheme(&scheme.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(format!("unsupported plugin stream protocol: {scheme}"))
    }
}

pub(super) fn is_supported_capability_kind(kind: &str) -> bool {
    matches!(
        kind,
        "subtitleStyle" | "capture" | "streamSource" | "aiTranscription" | "aiTranslation"
    )
}

pub(super) fn is_supported_plugin_permission(permission: &str) -> bool {
    matches!(
        permission,
        "mpv.subtitleStyle"
            | "mpv.loadOptions"
            | "mpv.capture"
            | "mpv.wall"
            | "media.openStream"
            | "filesystem.pick"
            | "filesystem.reveal"
            | "network.request"
            | "ai.transcribe"
            | "ai.translate"
    )
}

pub(super) fn is_supported_setting_kind(kind: &str) -> bool {
    matches!(
        kind,
        "boolean" | "number" | "text" | "select" | "color" | "directory"
    )
}

pub(super) fn is_supported_setting_placement(placement: &str) -> bool {
    matches!(
        placement,
        "pluginSettings"
            | "subtitleSettings"
            | "captureSettings"
            | "streamSettings"
            | "controls.left"
            | "controls.center"
            | "controls.right"
            | "contextMenu"
            | "overlay.status"
            | "playlist.actions"
    )
}

pub(super) fn is_supported_action_placement(placement: &str) -> bool {
    matches!(
        placement,
        "controls.left"
            | "controls.center"
            | "controls.right"
            | "contextMenu"
            | "overlay.status"
            | "playlist.actions"
    )
}

pub(super) fn is_supported_plugin_action_command(command: &str) -> bool {
    is_plugin_runtime_action_command(command)
        || matches!(
            command,
            "player.openMedia"
                | "player.openStream"
                | "player.openStreamDialog"
                | "player.captureScreenshot"
                | "player.startRecording"
                | "player.stopRecording"
                | "player.toggleRecording"
                | "player.togglePlayback"
                | "player.stop"
                | "player.restart"
                | "player.togglePlaylist"
                | "player.toggleTracks"
                | "player.toggleLoop"
                | "player.toggleSpeed"
                | "window.toggleFullscreen"
                | "window.toggleAlwaysOnTop"
                | "app.openSettings"
        )
}

pub(super) fn is_plugin_runtime_action_command(command: &str) -> bool {
    command.len() <= 96
        && command.starts_with("plugin.")
        && validate_dotted_identifier("plugin action command", command, true).is_ok()
}

pub(super) fn plugin_action_required_permission(command: &str) -> Option<&'static str> {
    match command {
        "player.captureScreenshot"
        | "player.startRecording"
        | "player.stopRecording"
        | "player.toggleRecording" => Some("mpv.capture"),
        "player.openStream" | "player.openStreamDialog" => Some("media.openStream"),
        _ => None,
    }
}

pub(super) fn is_supported_plugin_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

pub(super) fn is_supported_plugin_action_icon(icon: &str) -> bool {
    matches!(
        icon,
        "folder"
            | "folderAdd"
            | "play"
            | "pause"
            | "stop"
            | "restart"
            | "list"
            | "tracks"
            | "settings"
            | "fullscreen"
            | "pin"
            | "plugin"
            | "camera"
            | "record"
            | "stream"
            | "info"
    )
}

pub(super) fn validate_plugin_mpv_property(property: &str) -> Result<(), String> {
    if is_allowed_plugin_mpv_property(property) {
        Ok(())
    } else {
        Err(format!("unsupported plugin mpv property: {property}"))
    }
}

pub(super) fn is_allowed_plugin_mpv_property(property: &str) -> bool {
    matches!(
        property,
        "sub-font"
            | "sub-font-size"
            | "sub-scale"
            | "sub-pos"
            | "sub-color"
            | "sub-spacing"
            | "sub-outline-size"
            | "sub-border-size"
            | "sub-shadow-offset"
    )
}

pub(super) fn validate_localized_text_map(
    label: &str,
    values: &HashMap<String, String>,
    max_len: usize,
) -> Result<(), String> {
    for (locale, text) in values {
        validate_locale_key(label, locale)?;
        validate_non_empty(label, text)?;
        if text.len() > max_len {
            return Err(format!("{label} value is too long"));
        }
    }
    Ok(())
}

pub(super) fn validate_locale_key(label: &str, locale: &str) -> Result<(), String> {
    if locale.is_empty()
        || locale.len() > 16
        || !locale
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
    {
        return Err(format!("{label} contains an invalid locale key: {locale}"));
    }
    Ok(())
}

pub(super) fn validate_theme_manifest(theme: &ThemeManifest) -> Result<(), String> {
    validate_non_empty("theme id", &theme.id)?;
    validate_non_empty("theme name", &theme.name)?;
    validate_non_empty("theme version", &theme.version)?;
    validate_dotted_identifier("theme id", &theme.id, false)?;
    validate_simple_semver("theme version", &theme.version)?;
    validate_theme_tokens(&theme.tokens)
}

pub(super) fn validate_theme_tokens(tokens: &ThemeTokens) -> Result<(), String> {
    validate_color_token("surface", &tokens.surface)?;
    validate_color_token("panel", &tokens.panel)?;
    validate_color_token("panelStrong", &tokens.panel_strong)?;
    validate_color_token("text", &tokens.text)?;
    validate_color_token("muted", &tokens.muted)?;
    validate_color_token("faint", &tokens.faint)?;
    validate_color_token("accent", &tokens.accent)?;
    validate_color_token("danger", &tokens.danger)?;
    validate_color_token("line", &tokens.line)?;
    validate_color_token("control", &tokens.control)?;
    validate_color_token("scrollbarThumb", &tokens.scrollbar_thumb)?;
    validate_color_token("scrollbarThumbHover", &tokens.scrollbar_thumb_hover)
}

pub(super) fn validate_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} cannot be empty"))
    } else {
        Ok(())
    }
}

pub(super) fn validate_dotted_identifier(
    label: &str,
    value: &str,
    require_dot: bool,
) -> Result<(), String> {
    if require_dot && !value.contains('.') {
        return Err(format!("{label} must use a dotted identifier"));
    }
    if value.split('.').all(is_identifier_segment) {
        Ok(())
    } else {
        Err(format!("{label} is invalid: {value}"))
    }
}

pub(super) fn is_identifier_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-')
}

pub(super) fn validate_simple_semver(label: &str, value: &str) -> Result<(), String> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|char| char.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err(format!("{label} must use major.minor.patch"))
    }
}

pub(super) fn parse_simple_semver(value: &str) -> Result<[u64; 3], String> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() != 3 {
        return Err(format!("invalid semver: {value}"));
    }
    let major = parts[0]
        .parse::<u64>()
        .map_err(|_| format!("invalid semver: {value}"))?;
    let minor = parts[1]
        .parse::<u64>()
        .map_err(|_| format!("invalid semver: {value}"))?;
    let patch = parts[2]
        .parse::<u64>()
        .map_err(|_| format!("invalid semver: {value}"))?;
    Ok([major, minor, patch])
}

pub(super) fn compare_simple_semver(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    Ok(parse_simple_semver(left)?.cmp(&parse_simple_semver(right)?))
}

pub(super) fn validate_http_url(label: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    validate_non_empty(label, trimmed)?;
    if trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err(format!("{label} is invalid"));
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err(format!("{label} must use http or https"));
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return Err(format!("{label} must use http or https"));
    }
    if rest.trim_matches('/').is_empty() {
        return Err(format!("{label} must include a host"));
    }
    Ok(())
}

pub(super) fn validate_color_token(token: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if is_hex_color(value) || is_rgba_color(value) {
        Ok(())
    } else {
        Err(format!("{token} color is invalid: {value}"))
    }
}

pub(super) fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|char| char.is_ascii_hexdigit())
}

pub(super) fn is_rgba_color(value: &str) -> bool {
    let Some(inner) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return false;
    }

    let rgb_ok = parts[..3]
        .iter()
        .all(|part| part.parse::<u16>().is_ok_and(|value| value <= 255));
    let alpha_ok = parts[3]
        .parse::<f64>()
        .is_ok_and(|value| (0.0..=1.0).contains(&value));
    rgb_ok && alpha_ok
}
