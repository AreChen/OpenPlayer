use super::*;

#[test]
fn maps_track_kinds_to_mpv_properties() {
    assert_eq!(track_property_for_kind("audio").unwrap(), "aid");
    assert_eq!(track_property_for_kind("video").unwrap(), "vid");
    assert_eq!(track_property_for_kind("subtitle").unwrap(), "sid");
    assert_eq!(track_property_for_kind("sub").unwrap(), "sid");
    assert_eq!(
        track_property_for_kind("chapter").expect_err("unsupported kinds should be rejected"),
        "invalid mpv track kind"
    );
}

#[test]
fn normalizes_plugin_owned_mpv_properties() {
    assert_eq!(
        normalize_plugin_mpv_property("sub-font-size", &serde_json::json!(52)).unwrap(),
        ("sub-font-size", PluginMpvPropertyValue::Number(52.0))
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-font", &serde_json::json!("Inter")).unwrap(),
        (
            "sub-font",
            PluginMpvPropertyValue::Text("Inter".to_string())
        )
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-color", &serde_json::json!("#78d5b3")).unwrap(),
        (
            "sub-color",
            PluginMpvPropertyValue::Text("#78d5b3".to_string())
        )
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(4)).unwrap(),
        ("sub-spacing", PluginMpvPropertyValue::Text("4".to_string()))
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(10)).unwrap(),
        (
            "sub-spacing",
            PluginMpvPropertyValue::Text("10".to_string())
        )
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-border-size", &serde_json::json!(2.5)).unwrap(),
        ("sub-outline-size", PluginMpvPropertyValue::Number(2.5))
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-shadow-offset", &serde_json::json!(1.5)).unwrap(),
        ("sub-shadow-offset", PluginMpvPropertyValue::Number(1.5))
    );
}

#[test]
fn rejects_plugin_owned_mpv_properties_outside_allowlist() {
    assert_eq!(
        normalize_plugin_mpv_property("vf", &serde_json::json!("lavfi=[scale=2]"))
            .expect_err("plugins must not set arbitrary mpv properties"),
        "unsupported plugin mpv property: vf"
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-font-size", &serde_json::json!(999))
            .expect_err("subtitle font size outside the allowed range should be rejected"),
        "invalid plugin subtitle font size"
    );
    assert_eq!(
        normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(11))
            .expect_err("subtitle spacing above mpv's stable range should be rejected"),
        "invalid plugin subtitle spacing"
    );
}

#[test]
fn plugin_subtitle_style_properties_force_ass_overrides() {
    assert!(plugin_subtitle_style_requires_ass_override("sub-font-size"));
    assert!(plugin_subtitle_style_requires_ass_override("sub-spacing"));
    assert!(!plugin_subtitle_style_requires_ass_override(
        "sub-line-spacing"
    ));
    assert!(!plugin_subtitle_style_requires_ass_override("sub-delay"));
}

#[test]
fn subtitle_spacing_writes_only_stable_mpv_property() {
    assert_eq!(
        plugin_mpv_property_write_targets("sub-line-spacing"),
        &[] as &[&str]
    );
    assert_eq!(
        plugin_mpv_property_write_targets("sub-spacing"),
        &["sub-spacing"]
    );
}
