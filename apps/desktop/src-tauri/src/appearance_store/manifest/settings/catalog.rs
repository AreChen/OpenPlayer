pub(super) fn validate_setting_kind(kind: &str) -> Result<(), String> {
    if matches!(
        kind,
        "boolean" | "number" | "text" | "select" | "color" | "directory"
    ) {
        Ok(())
    } else {
        Err(format!("unsupported plugin setting kind: {kind}"))
    }
}

pub(super) fn validate_setting_placement(placement: &str) -> Result<(), String> {
    if matches!(
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
    ) {
        Ok(())
    } else {
        Err(format!("unsupported plugin setting placement: {placement}"))
    }
}
