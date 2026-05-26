use super::super::primitives::validate_dotted_identifier;

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
