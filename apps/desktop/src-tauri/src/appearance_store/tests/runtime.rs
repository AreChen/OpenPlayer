use super::*;
use std::collections::HashMap;

fn runtime_plugin_with_storage(version: u32, defaults: &str) -> String {
    webview_runtime_plugin_json().replace(
        r#""actions": ["#,
        &format!(
            r#""storage": {{
              "version": {version},
              "defaults": {{
                {defaults}
              }}
            }},
            "actions": ["#
        ),
    )
}

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
fn plugin_runtime_storage_batch_update_is_atomic() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("runtime plugin should import");
    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "queue.legacy",
            serde_json::json!({ "status": "pending" }),
        )
        .expect("initial runtime storage value should persist");

    let info = store
        .update_plugin_runtime_storage_values(
            "dev.openplayer.runtime.worker",
            HashMap::from([
                (
                    "queue.active".to_string(),
                    serde_json::json!({ "status": "running" }),
                ),
                ("settings.model".to_string(), serde_json::json!("small")),
            ]),
            vec!["queue.legacy".to_string()],
        )
        .expect("batch storage update should persist atomically");
    assert_eq!(info.keys, vec!["queue.active", "settings.model"]);
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "queue.legacy")
            .expect("removed runtime storage value should be readable"),
        None
    );
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "queue.active")
            .expect("batch runtime storage value should be readable"),
        Some(serde_json::json!({ "status": "running" }))
    );

    let error = store
        .update_plugin_runtime_storage_values(
            "dev.openplayer.runtime.worker",
            HashMap::from([(".bad".to_string(), serde_json::json!(true))]),
            vec!["queue.active".to_string()],
        )
        .expect_err("invalid batch storage keys should reject the whole transaction");
    assert!(error.contains("plugin runtime storage key is invalid"));
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "queue.active")
            .expect("failed batch update should leave existing values untouched"),
        Some(serde_json::json!({ "status": "running" }))
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn plugin_runtime_storage_lists_values_by_prefix_and_limit() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("runtime plugin should import");
    store
        .update_plugin_runtime_storage_values(
            "dev.openplayer.runtime.worker",
            HashMap::from([
                ("cache.source-a.0".to_string(), serde_json::json!("a0")),
                ("cache.source-a.1".to_string(), serde_json::json!("a1")),
                ("cache.source-b.0".to_string(), serde_json::json!("b0")),
                ("settings.mode".to_string(), serde_json::json!("compact")),
            ]),
            Vec::new(),
        )
        .expect("runtime storage values should persist");

    let values = store
        .plugin_runtime_storage_values_filtered(
            "dev.openplayer.runtime.worker",
            Some("cache.source-a."),
            Some(1),
        )
        .expect("runtime storage should support bounded prefix scans");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(
        values,
        HashMap::from([("cache.source-a.0".to_string(), serde_json::json!("a0"))])
    );
}

#[test]
fn plugin_runtime_storage_info_reports_size_limits_and_usage() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("runtime plugin should import");
    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "cache.meta",
            serde_json::json!({ "items": 2 }),
        )
        .expect("runtime storage value should persist");

    let info = store
        .plugin_runtime_storage_info("dev.openplayer.runtime.worker")
        .expect("runtime storage info should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(info.max_value_bytes, 64 * 1024);
    assert!(info.total_bytes >= "{\"items\":2}".len());
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
fn plugin_runtime_storage_manifest_initializes_defaults_and_tracks_schema_upgrades() {
    let (mut store, directory) = temp_store();
    let manifest = webview_runtime_plugin_json().replace(
        r#""actions": ["#,
        r#""storage": {
              "version": 1,
              "defaults": {
                "transcript.language": "auto",
                "transcript.queue": []
              }
            },
            "actions": ["#,
    );

    store
        .import_theme_plugin_json(&manifest)
        .expect("runtime plugin storage manifest should import");
    let info = store
        .plugin_runtime_storage_info("dev.openplayer.runtime.worker")
        .expect("runtime storage info should be readable");
    assert_eq!(info.schema_version, 1);
    assert_eq!(info.manifest_version, 1);
    assert_eq!(info.keys, vec!["transcript.language", "transcript.queue"]);
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "transcript.language")
            .expect("storage default should be readable"),
        Some(serde_json::json!("auto"))
    );

    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "transcript.language",
            serde_json::json!("zh-CN"),
        )
        .expect("plugin-owned value should be writable");
    let upgraded_manifest = webview_runtime_plugin_json().replace(
        r#""actions": ["#,
        r#""storage": {
              "version": 2,
              "defaults": {
                "transcript.language": "auto",
                "transcript.model": "small",
                "transcript.queue": []
              }
            },
            "actions": ["#,
    );

    store
        .import_theme_plugin_json(&upgraded_manifest)
        .expect("runtime plugin storage upgrade should import");
    let upgraded_info = store
        .plugin_runtime_storage_info("dev.openplayer.runtime.worker")
        .expect("upgraded runtime storage info should be readable");

    assert_eq!(upgraded_info.schema_version, 1);
    assert_eq!(upgraded_info.manifest_version, 2);
    assert_eq!(
        upgraded_info.keys,
        vec![
            "transcript.language",
            "transcript.model",
            "transcript.queue"
        ]
    );
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "transcript.language")
            .expect("existing plugin value should survive storage upgrade"),
        Some(serde_json::json!("zh-CN"))
    );
    assert_eq!(
        store
            .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "transcript.model")
            .expect("new storage default should be initialized"),
        Some(serde_json::json!("small"))
    );
    let migrated_info = store
        .mark_plugin_runtime_storage_migrated("dev.openplayer.runtime.worker", Some(2))
        .expect("plugin migration marker should persist");
    assert_eq!(migrated_info.schema_version, 2);
    assert_eq!(migrated_info.manifest_version, 2);
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn plugin_runtime_storage_schema_upgrade_from_legacy_storage_starts_at_zero() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("legacy runtime plugin should import");
    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "legacy.state",
            serde_json::json!({ "enabled": true }),
        )
        .expect("legacy plugin storage value should persist");

    let schema_manifest = webview_runtime_plugin_json().replace(
        r#""actions": ["#,
        r#""storage": {
              "version": 2,
              "defaults": {
                "legacy.schema": "v2"
              }
            },
            "actions": ["#,
    );
    store
        .import_theme_plugin_json(&schema_manifest)
        .expect("legacy plugin storage schema should import");
    let info = store
        .plugin_runtime_storage_info("dev.openplayer.runtime.worker")
        .expect("legacy storage info should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(info.schema_version, 0);
    assert_eq!(info.manifest_version, 2);
    assert_eq!(info.keys, vec!["legacy.schema", "legacy.state"]);
}

#[test]
fn rejects_invalid_plugin_runtime_storage_manifests() {
    let (mut store, directory) = temp_store();
    let invalid_version = runtime_plugin_with_storage(0, r#""state": "new""#);
    assert!(
        store
            .import_theme_plugin_json(&invalid_version)
            .expect_err("zero storage schema version should be rejected")
            .contains("plugin storage version must be at least 1")
    );

    let invalid_key = runtime_plugin_with_storage(1, r#""..bad": true"#);
    assert!(
        store
            .import_theme_plugin_json(&invalid_key)
            .expect_err("invalid storage default key should be rejected")
            .contains("plugin runtime storage key is invalid")
    );

    let too_many_defaults = (0..257)
        .map(|index| format!(r#""key.{index}": {index}"#))
        .collect::<Vec<_>>()
        .join(",");
    let invalid_defaults = runtime_plugin_with_storage(1, &too_many_defaults);
    let error = store
        .import_theme_plugin_json(&invalid_defaults)
        .expect_err("oversized storage defaults should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("plugin storage defaults define too many keys"));
}

#[test]
fn rejects_invalid_plugin_runtime_storage_migration_targets() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(webview_runtime_plugin_json())
        .expect("runtime plugin should import");
    assert!(
        store
            .mark_plugin_runtime_storage_migrated("dev.openplayer.runtime.worker", None)
            .expect_err("plugins without storage schema should not mark migrations")
            .contains("plugin does not declare a storage schema")
    );

    let schema_manifest = runtime_plugin_with_storage(2, r#""state": "new""#);
    store
        .import_theme_plugin_json(&schema_manifest)
        .expect("storage schema plugin should import");
    assert!(
        store
            .mark_plugin_runtime_storage_migrated("dev.openplayer.runtime.worker", Some(3))
            .expect_err("migration target must not exceed manifest storage version")
            .contains("exceeds manifest version 2")
    );
    assert!(
        store
            .mark_plugin_runtime_storage_migrated("dev.openplayer.runtime.worker", Some(1))
            .expect_err("migration target must not move backward")
            .contains("older than current schema version 2")
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn uninstalling_plugin_clears_runtime_storage_metadata_before_reinstall() {
    let (mut store, directory) = temp_store();
    let schema_v2 = runtime_plugin_with_storage(2, r#""state": "new""#);
    store
        .import_theme_plugin_json(&schema_v2)
        .expect("storage schema plugin should import");
    store
        .set_plugin_runtime_storage_value(
            "dev.openplayer.runtime.worker",
            "state",
            serde_json::json!("custom"),
        )
        .expect("runtime storage value should persist");
    store
        .mark_plugin_runtime_storage_migrated("dev.openplayer.runtime.worker", Some(2))
        .expect("schema v2 marker should persist");

    store
        .uninstall_plugin("dev.openplayer.runtime.worker")
        .expect("plugin uninstall should succeed");
    assert!(
        store
            .plugin_runtime_storage_values("dev.openplayer.runtime.worker")
            .expect("runtime storage scan should succeed after uninstall")
            .is_empty()
    );

    let schema_v1 = runtime_plugin_with_storage(1, r#""state": "fresh""#);
    store
        .import_theme_plugin_json(&schema_v1)
        .expect("storage schema plugin should reinstall cleanly");
    let info = store
        .plugin_runtime_storage_info("dev.openplayer.runtime.worker")
        .expect("reinstalled storage info should be readable");
    let value = store
        .plugin_runtime_storage_value("dev.openplayer.runtime.worker", "state")
        .expect("reinstalled storage default should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(info.schema_version, 1);
    assert_eq!(info.manifest_version, 1);
    assert_eq!(info.keys, vec!["state"]);
    assert_eq!(value, Some(serde_json::json!("fresh")));
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
    assert!(is_supported_plugin_permission("media.export"));
    assert!(is_supported_plugin_permission("audio.extract"));
    assert!(is_supported_plugin_permission("subtitle.read"));
    assert!(is_supported_plugin_permission("subtitle.write"));
}

#[test]
fn rejects_reserved_ai_feature_permissions() {
    assert!(!is_supported_plugin_permission(
        &["ai", "transcribe"].join(".")
    ));
    assert!(!is_supported_plugin_permission(
        &["ai", "translate"].join(".")
    ));
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
            "events": ["media.loaded", "playback.started", "playlist.changed", "recording.changed", "theme.changed"]"#,
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
        vec![
            "media.loaded",
            "playback.started",
            "playlist.changed",
            "recording.changed",
            "theme.changed"
        ]
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
