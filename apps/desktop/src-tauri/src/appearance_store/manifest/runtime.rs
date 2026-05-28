use super::primitives::{
    validate_dotted_identifier, validate_localized_text_map, validate_non_empty,
};
use crate::appearance_store::{
    records::runtime_kind_label,
    types::{PluginCapabilityManifest, PluginRuntime, PluginRuntimeKind, PluginViewManifest},
};

pub(super) fn validate_plugin_runtime(runtime: &PluginRuntime) -> Result<(), String> {
    for event in &runtime.events {
        if !is_supported_plugin_runtime_event(event) {
            return Err(format!("unsupported plugin runtime event: {event}"));
        }
    }

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

pub(super) fn validate_plugin_view(view: &PluginViewManifest) -> Result<(), String> {
    validate_non_empty("plugin view id", &view.id)?;
    validate_non_empty("plugin view title", &view.title)?;
    validate_dotted_identifier("plugin view id", &view.id, false)?;
    validate_relative_plugin_entry(&view.entry)?;
    if let Some(description) = view.description.as_deref() {
        validate_non_empty("plugin view description", description)?;
    }
    if !matches!(view.presentation.as_str(), "overlay" | "sidePanel") {
        return Err(format!(
            "unsupported plugin view presentation: {}",
            view.presentation
        ));
    }
    if let Some(setting_id) = view.frame_opacity_setting.as_deref() {
        validate_dotted_identifier("plugin view frameOpacitySetting", setting_id, false)?;
        if view.presentation != "sidePanel" {
            return Err(
                "plugin view frameOpacitySetting requires sidePanel presentation".to_string(),
            );
        }
    }
    validate_localized_text_map("plugin view titleI18n", &view.title_i18n, 128)?;
    validate_localized_text_map("plugin view descriptionI18n", &view.description_i18n, 512)?;
    Ok(())
}

fn is_supported_capability_kind(kind: &str) -> bool {
    matches!(
        kind,
        "subtitleStyle" | "capture" | "streamSource" | "audioTool" | "subtitleTool" | "mpvControl"
    )
}

pub(super) fn is_supported_plugin_permission(permission: &str) -> bool {
    matches!(
        permission,
        "mpv.subtitleStyle"
            | "mpv.loadOptions"
            | "mpv.capture"
            | "mpv.wall"
            | "mpv.core"
            | "mpv.filters"
            | "mpv.osd"
            | "mpv.scriptMessage"
            | "media.openStream"
            | "filesystem.pick"
            | "filesystem.reveal"
            | "network.request"
            | "audio.extract"
            | "subtitle.write"
    )
}

fn is_supported_plugin_runtime_event(event: &str) -> bool {
    matches!(
        event,
        "app.ready"
            | "media.opening"
            | "media.loaded"
            | "playback.snapshot"
            | "playback.started"
            | "playback.paused"
            | "playback.ended"
            | "playback.stopped"
            | "playback.seeked"
            | "playback.volumeChanged"
            | "playback.speedChanged"
            | "tracks.changed"
            | "theme.changed"
            | "window.fullscreenChanged"
            | "plugin.view.opened"
            | "plugin.view.closed"
    )
}
