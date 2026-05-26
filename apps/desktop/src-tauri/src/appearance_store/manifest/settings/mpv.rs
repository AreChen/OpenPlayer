pub(super) fn validate_plugin_mpv_property(property: &str) -> Result<(), String> {
    if is_allowed_plugin_mpv_property(property) {
        Ok(())
    } else {
        Err(format!("unsupported plugin mpv property: {property}"))
    }
}

pub(in crate::appearance_store) fn is_allowed_plugin_mpv_property(property: &str) -> bool {
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
