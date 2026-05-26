use super::*;

#[test]
fn imports_plugin_actions_for_ui_slots() {
    let (mut store, directory) = temp_store();

    let state = store
        .import_theme_plugin_json(action_plugin_json())
        .expect("action plugin manifest should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.quick.actions")
        .expect("action plugin should be listed");
    assert_eq!(plugin.action_count, 2);
    assert_eq!(plugin.actions[0].id, "toggle-tracks");
    assert_eq!(plugin.actions[0].placement, "controls.right");
    assert_eq!(plugin.actions[0].command, "player.toggleTracks");
    assert!(plugin.actions[0].requires_media);
    assert_eq!(plugin.actions[1].placement, "contextMenu");
}

#[test]
fn imports_capability_actions_with_valid_permissions_and_args() {
    let (mut store, directory) = temp_store();

    let state = store
        .import_theme_plugin_json(capability_action_plugin_json())
        .expect("capability actions should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.capability.actions")
        .expect("capability action plugin should be listed");
    assert_eq!(plugin.action_count, 4);
    assert!(
        plugin
            .settings
            .iter()
            .any(|setting| { setting.id == "capture-directory" && setting.kind == "directory" })
    );
    assert_eq!(plugin.actions[0].command, "player.captureScreenshot");
    assert_eq!(
        plugin.actions[0].args,
        serde_json::json!({ "openFolder": true, "directorySetting": "capture-directory" })
    );
    assert_eq!(plugin.actions[1].command, "player.openStream");
    assert_eq!(
        plugin.actions[1].args,
        serde_json::json!({ "url": "https://example.com/live.m3u8", "name": "Live Stream" })
    );
    assert_eq!(plugin.actions[2].command, "player.openStreamDialog");
    assert!(plugin.actions[2].args.is_object());
    assert_eq!(plugin.actions[3].command, "player.toggleRecording");
    assert_eq!(
        plugin.actions[3].args,
        serde_json::json!({ "formatSetting": "recording-format", "directorySetting": "capture-directory", "openFolderSetting": "open-folder-after-capture" })
    );
}

#[test]
fn rejects_capability_actions_without_required_permissions() {
    let (mut store, directory) = temp_store();
    let invalid = capability_action_plugin_json()
        .replace("\"permissions\": [\"mpv.capture\"]", "\"permissions\": []");

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("capture action without permission should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("requires permission mpv.capture"));
}

#[test]
fn rejects_plugin_stream_actions_with_unsafe_urls() {
    let (mut store, directory) = temp_store();
    let invalid = capability_action_plugin_json()
        .replace("https://example.com/live.m3u8", "file://C:/secret.mp4");

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("unsafe stream urls should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("unsupported plugin stream protocol"));
}

#[test]
fn rejects_plugin_actions_with_unsupported_commands() {
    let (mut store, directory) = temp_store();
    let invalid = action_plugin_json().replace("player.toggleTracks", "system.exec");

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("unsupported action commands should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("unsupported plugin action command"));
}
