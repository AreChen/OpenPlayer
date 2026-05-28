use super::*;

#[test]
fn plugin_runtime_storage_is_isolated_and_persistent() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("runtime plugin should import");
    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "history.last",
            serde_json::json!({ "url": "https://example.com/live.m3u8" }),
        )
        .expect("runtime storage value should persist");
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "history.last")
            .expect("runtime storage should be readable"),
        Some(serde_json::json!({ "url": "https://example.com/live.m3u8" }))
    );
    drop(store);

    let reopened =
        AppearanceStore::open(directory.join("settings.redb")).expect("store should reopen");
    let values = reopened
        .plugin_runtime_storage_values("dev.openplayer.runtime.worker")
        .expect("runtime storage list should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(
        values.get("history.last"),
        Some(&serde_json::json!({ "url": "https://example.com/live.m3u8" }))
    );
}

#[test]
fn plugin_runtime_storage_is_removed_with_plugin() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("runtime plugin should import");
    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "state",
            serde_json::json!("enabled"),
        )
        .expect("runtime storage value should persist");

    store
        .uninstall_plugin("dev.openplayer.runtime.worker")
        .expect("plugin uninstall should succeed");
    let values = store
        .plugin_runtime_storage_values("dev.openplayer.runtime.worker")
        .expect("runtime storage scan should succeed after uninstall");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(values.is_empty());
}

#[test]
fn imports_webview_runtime_source_from_installed_plugin_package() {
    let (mut store, directory) = temp_store();
    let source_directory = directory.join("worker-runtime");
    std::fs::create_dir_all(source_directory.join("dist"))
        .expect("runtime package directory should be created");
    std::fs::write(
        source_directory.join("manifest.json"),
        webview_runtime_plugin_json(),
    )
    .expect("runtime plugin manifest should be written");
    std::fs::write(
        source_directory.join("dist").join("plugin.js"),
        "openplayer.request('player.captureScreenshot', { openFolder: false });",
    )
    .expect("runtime plugin script should be written");

    store
        .import_plugin_directory_path(&source_directory)
        .expect("webview runtime plugin should import");
    let sources = store
        .plugin_runtime_sources()
        .expect("runtime sources should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].plugin_id, "dev.openplayer.runtime.worker");
    assert_eq!(sources[0].entry, "dist/plugin.js");
    assert!(sources[0].script.contains("player.captureScreenshot"));
    assert_eq!(sources[0].permissions, vec!["mpv.capture"]);
}

#[test]
fn accepts_plugin_load_options_permission_for_runtime_hooks() {
    assert!(is_supported_plugin_permission("mpv.loadOptions"));
}

#[test]
fn accepts_phase_two_plugin_sdk_permissions() {
    assert!(is_supported_plugin_permission("mpv.wall"));
    assert!(is_supported_plugin_permission("mpv.core"));
    assert!(is_supported_plugin_permission("mpv.filters"));
    assert!(is_supported_plugin_permission("mpv.osd"));
    assert!(is_supported_plugin_permission("mpv.scriptMessage"));
    assert!(is_supported_plugin_permission("network.request"));
    assert!(is_supported_plugin_permission("filesystem.pick"));
    assert!(is_supported_plugin_permission("filesystem.reveal"));
}

#[test]
fn imports_mpv_control_plugin_capability_permissions() {
    let (mut store, directory) = temp_store();
    let manifest = webview_runtime_plugin_json()
        .replace(r#""kind": "capture""#, r#""kind": "mpvControl""#)
        .replace(
            r#""permissions": ["mpv.capture"]"#,
            r#""permissions": ["mpv.core", "mpv.filters", "mpv.osd", "mpv.scriptMessage"]"#,
        );

    let state = store
        .import_theme_plugin_json(&manifest)
        .expect("mpv control plugin capability should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.runtime.worker")
        .expect("plugin should be listed");
    assert_eq!(
        plugin.permissions,
        vec!["mpv.core", "mpv.filters", "mpv.osd", "mpv.scriptMessage"]
    );
}

#[test]
fn imports_runtime_event_subscription_manifest() {
    let (mut store, directory) = temp_store();
    let source_directory = directory.join("event-runtime");
    std::fs::create_dir_all(source_directory.join("dist"))
        .expect("runtime package directory should be created");
    let manifest = webview_runtime_plugin_json().replace(
        r#""sandbox": "openplayer-worker""#,
        r#""sandbox": "openplayer-worker",
            "events": ["media.loaded", "playback.started", "theme.changed"]"#,
    );
    std::fs::write(source_directory.join("manifest.json"), manifest)
        .expect("runtime plugin manifest should be written");
    std::fs::write(
        source_directory.join("dist").join("plugin.js"),
        "\"use strict\";",
    )
    .expect("runtime plugin script should be written");

    store
        .import_plugin_directory_path(&source_directory)
        .expect("runtime event subscriptions should import");
    let sources = store
        .plugin_runtime_sources()
        .expect("runtime sources should include event subscriptions");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(
        sources[0].events,
        vec!["media.loaded", "playback.started", "theme.changed"]
    );
}

#[test]
fn imports_runtime_owned_plugin_action_commands() {
    let (mut store, directory) = temp_store();
    let runtime_action = webview_runtime_plugin_json().replace(
        r#""command": "app.openSettings""#,
        r#""command": "plugin.show-info", "args": { "target": "runtime" }"#,
    );

    let state = store
        .import_theme_plugin_json(&runtime_action)
        .expect("plugin-owned runtime action should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.runtime.worker")
        .expect("runtime plugin should be listed");
    assert_eq!(plugin.actions[0].command, "plugin.show-info");
    assert_eq!(
        plugin.actions[0].args,
        serde_json::json!({ "target": "runtime" })
    );
}

#[test]
fn rejects_malformed_runtime_owned_plugin_action_commands() {
    let (mut store, directory) = temp_store();
    let invalid = webview_runtime_plugin_json().replace(
        r#""command": "app.openSettings""#,
        r#""command": "plugin.ShowInfo""#,
    );

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("malformed plugin-owned runtime action should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("unsupported plugin action command"));
}

#[test]
fn imports_plugin_sdk_metadata_author_and_update_url() {
    let (mut store, directory) = temp_store();
    let manifest = webview_runtime_plugin_json().replace(
        r#""version": "1.0.0","#,
        r#""version": "1.0.0",
          "apiVersion": "1",
          "minHostVersion": "1.3.0",
          "author": "OpenPlayer Team",
          "updateUrl": "https://github.com/AreChen/openplayer-plugins/releases","#,
    );

    let state = store
        .import_theme_plugin_json(&manifest)
        .expect("plugin SDK metadata should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.runtime.worker")
        .expect("runtime plugin should be listed");
    assert_eq!(plugin.api_version, "1");
    assert_eq!(plugin.min_host_version.as_deref(), Some("1.3.0"));
    assert_eq!(plugin.author.as_deref(), Some("OpenPlayer Team"));
    assert_eq!(
        plugin.update_url.as_deref(),
        Some("https://github.com/AreChen/openplayer-plugins/releases")
    );
}

#[test]
fn imports_plugin_custom_view_contributions() {
    let (mut store, directory) = temp_store();

    let state = store
        .import_theme_plugin_json(view_plugin_json())
        .expect("plugin views should import");
    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.view.wall")
        .expect("plugin summary should include the view plugin");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(plugin.views.len(), 1);
    assert_eq!(plugin.views[0].id, "wall");
    assert_eq!(plugin.views[0].entry, "views/wall.html");
    assert_eq!(plugin.views[0].presentation, "sidePanel");
    assert_eq!(
        plugin.views[0].frame_opacity_setting.as_deref(),
        Some("panel-opacity")
    );
    assert_eq!(
        plugin.views[0].title_i18n.get("zh-CN").map(String::as_str),
        Some("流媒体墙")
    );
}

#[test]
fn reads_installed_plugin_view_html_from_safe_package_path() {
    let (mut store, directory) = temp_store();
    let source_directory = directory.join("view-source");
    std::fs::create_dir_all(source_directory.join("runtime"))
        .expect("runtime directory should be created");
    std::fs::create_dir_all(source_directory.join("views"))
        .expect("view directory should be created");
    std::fs::write(source_directory.join("manifest.json"), view_plugin_json())
        .expect("manifest should be written");
    std::fs::write(
        source_directory.join("runtime").join("plugin.js"),
        "\"use strict\";",
    )
    .expect("runtime should be written");
    std::fs::write(
        source_directory.join("views").join("wall.html"),
        "<!doctype html><title>Wall</title>",
    )
    .expect("view HTML should be written");

    store
        .import_plugin_directory_path(&source_directory)
        .expect("view plugin directory should import");
    let view = store
        .plugin_view_html("dev.openplayer.view.wall", "wall")
        .expect("installed view HTML should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(view.plugin_id, "dev.openplayer.view.wall");
    assert_eq!(view.view_id, "wall");
    assert_eq!(view.title, "Stream Wall");
    assert!(view.html.contains("<title>Wall</title>"));
}

#[test]
fn rejects_plugin_custom_view_unsafe_entry_paths() {
    let (mut store, directory) = temp_store();
    let invalid = view_plugin_json().replace(
        "\"entry\": \"views/wall.html\"",
        "\"entry\": \"../wall.html\"",
    );

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("unsafe plugin view paths should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("relative package path"));
}

#[test]
fn rejects_plugin_custom_view_unknown_frame_opacity_setting() {
    let (mut store, directory) = temp_store();
    let invalid = view_plugin_json().replace(
        "\"frameOpacitySetting\": \"panel-opacity\"",
        "\"frameOpacitySetting\": \"missing-opacity\"",
    );

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("unknown plugin view frame opacity settings should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("frameOpacitySetting"));
}

#[test]
fn rejects_unsupported_plugin_sdk_metadata() {
    let (mut store, directory) = temp_store();
    let unsupported_api = webview_runtime_plugin_json().replace(
        r#""version": "1.0.0","#,
        r#""version": "1.0.0", "apiVersion": "99","#,
    );
    assert!(
        store
            .import_theme_plugin_json(&unsupported_api)
            .expect_err("unsupported plugin API version should be rejected")
            .contains("unsupported plugin apiVersion")
    );

    let future_host = webview_runtime_plugin_json().replace(
        r#""version": "1.0.0","#,
        r#""version": "1.0.0", "minHostVersion": "99.0.0","#,
    );
    assert!(
        store
            .import_theme_plugin_json(&future_host)
            .expect_err("future host requirement should be rejected")
            .contains("requires OpenPlayer 99.0.0")
    );

    let invalid_url = webview_runtime_plugin_json().replace(
        r#""version": "1.0.0","#,
        r#""version": "1.0.0", "updateUrl": "file:///unsafe","#,
    );
    assert!(
        store
            .import_theme_plugin_json(&invalid_url)
            .expect_err("unsafe update URL should be rejected")
            .contains("plugin updateUrl must use http or https")
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn rejects_webview_runtime_without_entry() {
    let (mut store, directory) = temp_store();
    let invalid = webview_runtime_plugin_json().replace(
        "\"entry\": \"dist/plugin.js\",\n            \"sandbox\": \"openplayer-worker\"",
        "\"sandbox\": \"openplayer-worker\"",
    );

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("webview runtime without entry should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("webviewJs"));
    assert!(error.contains("entry"));
}

#[test]
fn rejects_wasm_plugin_runtimes_until_wasm_sandbox_exists() {
    let (mut store, directory) = temp_store();

    let error = store
        .import_theme_plugin_json(wasm_runtime_plugin_json())
        .expect_err("wasm plugin runtimes should be rejected for now");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("wasm"));
    assert!(error.contains("not supported yet"));
}
