use super::*;
use serde_json::json;

#[test]
fn plugin_core_properties_are_typed_and_bounded() {
    let volume = normalize_plugin_core_property("volume", &json!(125.0))
        .expect("volume is a safe plugin property");
    assert_eq!(volume.property, "volume");
    assert_eq!(volume.value, PluginMpvCoreValue::Number(125.0));

    let pause = normalize_plugin_core_property("pause", &json!(true))
        .expect("pause is a safe plugin property");
    assert_eq!(pause.value, PluginMpvCoreValue::Bool(true));

    assert!(
        normalize_plugin_core_property("volume", &json!(250.0)).is_err(),
        "plugins must not bypass OpenPlayer volume limits",
    );
    assert!(
        normalize_plugin_core_property("input-ipc-server", &json!("\\\\.\\pipe\\mpv")).is_err(),
        "plugins must not write unsafe libmpv process properties",
    );
}

#[test]
fn plugin_core_commands_are_allowlisted_and_normalized() {
    let show_text = normalize_plugin_core_command("show-text", &json!(["Camera online", 1500]))
        .expect("show-text is a safe OSD command");
    assert_eq!(show_text.command, "show-text");
    assert_eq!(show_text.args, vec!["Camera online", "1500"]);

    let script_message =
        normalize_plugin_core_command("script-message", &json!(["openplayer-plugin", "refresh"]))
            .expect("script-message is safe when explicitly permissioned");
    assert_eq!(script_message.args, vec!["openplayer-plugin", "refresh"]);

    assert!(
        normalize_plugin_core_command("loadfile", &json!(["file:///C:/secret.mp4"])).is_err(),
        "plugins must not bypass OpenPlayer media opening",
    );
    assert!(
        normalize_plugin_core_command("run", &json!(["powershell"])).is_err(),
        "plugins must never execute shell-like mpv commands",
    );
}

#[test]
fn plugin_video_filters_are_scoped_to_plugin_owned_labels() {
    let filter = normalize_plugin_video_filter(
        "dev.openplayer.test-plugin",
        "tone",
        "eq",
        &json!({ "brightness": 10.0, "contrast": -5.0 }),
    )
    .expect("eq is an allowed plugin video filter");

    assert_eq!(filter.label, "op_dev_openplayer_test_plugin_tone");
    assert_eq!(
        filter.expression,
        "@op_dev_openplayer_test_plugin_tone:eq=brightness=10:contrast=-5",
    );

    assert!(
        normalize_plugin_video_filter(
            "dev.openplayer.test-plugin",
            "movie",
            "lavfi",
            &json!({ "graph": "movie=C:/secret.mp4" }),
        )
        .is_err(),
        "plugins must not use arbitrary filter graphs that can read external files",
    );
}

#[test]
fn plugin_audio_filters_are_scoped_to_plugin_owned_labels() {
    let filter = normalize_plugin_audio_filter(
        "dev.openplayer.test-plugin",
        "gain",
        "volume",
        &json!({ "gainDb": 3.5 }),
    )
    .expect("volume is an allowed plugin audio filter");

    assert_eq!(filter.label, "op_dev_openplayer_test_plugin_gain");
    assert_eq!(
        filter.expression,
        "@op_dev_openplayer_test_plugin_gain:volume=volume=3.5dB",
    );

    assert!(
        normalize_plugin_audio_filter(
            "dev.openplayer.test-plugin",
            "graph",
            "lavfi",
            &json!({ "graph": "amovie=C:/secret.wav" }),
        )
        .is_err(),
        "plugins must not use arbitrary audio filter graphs",
    );
}

#[test]
fn plugin_core_expands_safe_navigation_and_ab_loop_properties() {
    let ab_loop_a = normalize_plugin_core_property("ab-loop-a", &json!(12.5))
        .expect("AB loop start is a safe bounded property");
    assert_eq!(ab_loop_a.value, PluginMpvCoreValue::Number(12.5));
    assert!(normalize_plugin_core_property("ab-loop-b", &json!(-1.0)).is_err());

    assert!(normalize_plugin_core_command("chapter-next", &json!([])).is_ok());
    assert!(normalize_plugin_core_command("chapter-prev", &json!([])).is_ok());
    assert!(normalize_plugin_core_command("playlist-shuffle", &json!([])).is_ok());
}
