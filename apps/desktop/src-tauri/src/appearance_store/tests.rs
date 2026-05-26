use super::manifest::is_supported_plugin_permission;
use super::store::AppearanceStore;
use super::types::{PlayerPreferences, ThemeCatalogItem};
use super::*;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

static TEMP_STORE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_store() -> (AppearanceStore, PathBuf) {
    let counter = TEMP_STORE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "openplayer-appearance-{}-{}-{}",
        std::process::id(),
        counter,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("temp appearance directory should be created");
    let store = AppearanceStore::open(directory.join("settings.redb"))
        .expect("appearance store should open");
    (store, directory)
}

fn ocean_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.theme.ocean",
          "name": "Ocean Theme Pack",
          "version": "1.0.0",
          "description": "Ocean themes for OpenPlayer.",
          "entry": "manifest",
          "contributes": {
            "themes": [
              {
                "id": "dev.openplayer.theme.ocean.dark",
                "name": "Ocean Dark",
                "version": "1.0.0",
                "tokens": {
                  "surface": "#050607",
                  "panel": "rgba(8, 10, 12, 0.72)",
                  "panelStrong": "rgba(8, 10, 12, 0.88)",
                  "text": "#ece7dd",
                  "muted": "#b9b0a3",
                  "faint": "#8f867a",
                  "accent": "#62c7b7",
                  "danger": "#d78372",
                  "line": "rgba(236, 231, 221, 0.12)",
                  "control": "rgba(18, 21, 25, 0.72)",
                  "scrollbarThumb": "rgba(236, 231, 221, 0.22)",
                  "scrollbarThumbHover": "rgba(98, 199, 183, 0.46)"
                }
              }
            ]
          }
        }"##
}

fn subtitle_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.subtitle.styler",
          "name": "Subtitle Styler",
          "version": "1.0.0",
          "description": "Subtitle typography controls for OpenPlayer.",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "subtitle-style",
                "name": "Subtitle Styling",
                "kind": "subtitleStyle",
                "description": "Controls allowed subtitle mpv properties.",
                "permissions": ["mpv.subtitleStyle"]
              }
            ],
            "settings": [
              {
                "id": "font-size",
                "label": "Font Size",
                "description": "Subtitle font size in screen-scaled points.",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 42,
                "min": 12,
                "max": 96,
                "step": 1,
                "mpvProperty": "sub-font-size"
              },
              {
                "id": "font-family",
                "label": "Font Family",
                "kind": "text",
                "placement": "subtitleSettings",
                "defaultValue": "sans-serif",
                "mpvProperty": "sub-font"
              }
            ]
          }
        }"##
}

fn extended_subtitle_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.subtitle.typography",
          "name": "Subtitle Typography",
          "version": "1.0.0",
          "description": "Extended subtitle typography controls for OpenPlayer.",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "subtitle-style",
                "name": "Subtitle Styling",
                "kind": "subtitleStyle",
                "description": "Controls allowed subtitle mpv properties.",
                "permissions": ["mpv.subtitleStyle"]
              }
            ],
            "settings": [
              {
                "id": "letter-spacing",
                "label": "Letter Spacing",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 0,
                "min": -10,
                "max": 10,
                "step": 1,
                "mpvProperty": "sub-spacing"
              },
              {
                "id": "outline",
                "label": "Outline",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 3,
                "min": 0,
                "max": 12,
                "step": 0.5,
                "mpvProperty": "sub-outline-size"
              },
              {
                "id": "shadow",
                "label": "Shadow",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 1,
                "min": 0,
                "max": 12,
                "step": 0.5,
                "mpvProperty": "sub-shadow-offset"
              }
            ]
          }
        }"##
}

fn webview_runtime_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.runtime.worker",
          "name": "Worker Runtime",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "webviewJs",
            "entry": "dist/plugin.js",
            "sandbox": "openplayer-worker"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "capture",
                "name": "Capture",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              }
            ],
            "actions": [
              {
                "id": "runtime-info",
                "label": "Runtime Info",
                "placement": "contextMenu",
                "command": "app.openSettings",
                "icon": "plugin"
              }
            ]
          }
        }"##
}

fn view_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.view.wall",
          "name": "View Wall",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "webviewJs",
            "entry": "runtime/plugin.js",
            "sandbox": "openplayer-worker"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "stream-wall",
                "name": "Stream Wall",
                "kind": "streamSource",
                "permissions": ["network.request"]
              }
            ],
            "actions": [
              {
                "id": "open-wall",
                "label": "Open Wall",
                "placement": "contextMenu",
                "command": "plugin.open-wall",
                "icon": "stream"
              }
            ],
            "views": [
              {
                "id": "wall",
                "title": "Stream Wall",
                "entry": "views/wall.html",
                "description": "A custom plugin view.",
                "titleI18n": {
                  "zh-CN": "流媒体墙"
                }
              }
            ]
          }
        }"##
}

fn wasm_runtime_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.runtime.wasm",
          "name": "Wasm Runtime",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "wasm",
            "entry": "plugin.wasm",
            "sandbox": "openplayer-wasm"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "capture",
                "name": "Capture",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              }
            ]
          }
        }"##
}

fn action_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.quick.actions",
          "name": "Quick Actions",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "quick-controls",
                "name": "Quick Controls",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              }
            ],
            "actions": [
              {
                "id": "toggle-tracks",
                "label": "Tracks",
                "description": "Open the track and subtitle panel.",
                "placement": "controls.right",
                "command": "player.toggleTracks",
                "icon": "tracks",
                "requiresMedia": true
              },
              {
                "id": "open-settings",
                "label": "Settings",
                "placement": "contextMenu",
                "command": "app.openSettings",
                "icon": "settings"
              }
            ]
          }
        }"##
}

fn capability_action_plugin_json() -> &'static str {
    r##"{
          "id": "dev.openplayer.capability.actions",
          "name": "Capability Actions",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "capture",
                "name": "Capture",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              },
              {
                "id": "streams",
                "name": "Streams",
                "kind": "streamSource",
                "permissions": ["media.openStream"]
              }
            ],
            "settings": [
              {
                "id": "capture-directory",
                "label": "Save Directory",
                "kind": "directory",
                "placement": "captureSettings",
                "defaultValue": ""
              }
            ],
            "actions": [
              {
                "id": "screenshot",
                "label": "Screenshot",
                "placement": "controls.right",
                "command": "player.captureScreenshot",
                "icon": "camera",
                "requiresMedia": true,
                "args": {
                  "openFolder": true,
                  "directorySetting": "capture-directory"
                }
              },
              {
                "id": "open-stream",
                "label": "Open Stream",
                "placement": "playlist.actions",
                "command": "player.openStream",
                "icon": "stream",
                "args": {
                  "url": "https://example.com/live.m3u8",
                  "name": "Live Stream"
                }
              },
              {
                "id": "open-stream-dialog",
                "label": "Open Network Stream",
                "placement": "contextMenu",
                "command": "player.openStreamDialog",
                "icon": "stream"
              },
              {
                "id": "toggle-recording",
                "label": "Record",
                "placement": "controls.right",
                "command": "player.toggleRecording",
                "icon": "record",
                "requiresMedia": true,
                "args": {
                  "formatSetting": "recording-format",
                  "directorySetting": "capture-directory",
                  "openFolderSetting": "open-folder-after-capture"
                }
              }
            ]
          }
        }"##
}

fn write_opplugin_package(path: &Path, manifest_json: &str) {
    let file = File::create(path).expect("plugin package should be created");
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer
        .start_file(PLUGIN_MANIFEST_FILE, options)
        .expect("plugin package manifest entry should start");
    writer
        .write_all(manifest_json.as_bytes())
        .expect("plugin manifest should be written to package");
    writer
        .add_directory("assets/", options)
        .expect("plugin package asset directory should be added");
    writer
        .start_file("assets/readme.txt", options)
        .expect("plugin package asset entry should start");
    writer
        .write_all(b"package asset")
        .expect("plugin package asset should be written");
    writer.finish().expect("plugin package should finalize");
}

#[test]
fn redb_store_persists_theme_and_accent_override() {
    let (mut store, directory) = temp_store();
    store
        .set_accent_override(Some("#78d5b3".to_string()))
        .expect("valid accent should be persisted");
    drop(store);

    let store = AppearanceStore::open(directory.join("settings.redb"))
        .expect("appearance store should reopen");
    let state = store.state().expect("appearance state should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(state.active_theme_id, "studio-dark");
    assert_eq!(state.accent_override.as_deref(), Some("#78d5b3"));
}

#[test]
fn built_in_catalog_only_contains_studio_dark() {
    let (store, directory) = temp_store();

    let state = store.state().expect("appearance state should be readable");
    let built_ins: Vec<&ThemeCatalogItem> = state
        .themes
        .iter()
        .filter(|theme| theme.source == "builtIn")
        .collect();
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(built_ins.len(), 1);
    assert_eq!(built_ins[0].id, "studio-dark");
    assert_eq!(built_ins[0].name, "Studio Dark");
}

#[test]
fn imports_theme_plugin_and_lists_enabled_theme() {
    let (mut store, directory) = temp_store();

    let state = store
        .import_theme_plugin_json(ocean_plugin_json())
        .expect("theme plugin manifest should import");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(state.plugins.iter().any(|plugin| {
        plugin.id == "dev.openplayer.theme.ocean" && plugin.enabled && plugin.theme_count == 1
    }));
    assert!(state.themes.iter().any(|theme| {
        theme.id == "dev.openplayer.theme.ocean.dark"
            && theme.source == "plugin"
            && theme.plugin_id.as_deref() == Some("dev.openplayer.theme.ocean")
            && theme.enabled
    }));
}

#[test]
fn installs_manifest_file_into_managed_plugin_directory() {
    let (mut store, directory) = temp_store();
    let source_manifest = directory.join("source-subtitle-plugin.json");
    std::fs::write(&source_manifest, subtitle_plugin_json())
        .expect("source plugin manifest should be written");

    let state = store
        .import_plugin_manifest_path(&source_manifest)
        .expect("plugin manifest file should install");

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("installed plugin should be listed");
    let install_path = plugin
        .install_path
        .as_ref()
        .expect("installed plugin should expose install path");
    let installed_directory = PathBuf::from(install_path);

    assert_eq!(plugin.package_kind, "manifestFile");
    assert!(plugin.installed_at_ms.unwrap_or_default() > 0);
    assert!(installed_directory.ends_with("dev.openplayer.subtitle.styler"));
    assert!(installed_directory.join("manifest.json").exists());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn installs_plugin_directory_and_copies_package_assets() {
    let (mut store, directory) = temp_store();
    let source_directory = directory.join("subtitle-package");
    std::fs::create_dir_all(source_directory.join("assets"))
        .expect("source plugin package directory should be created");
    std::fs::write(
        source_directory.join("manifest.json"),
        subtitle_plugin_json(),
    )
    .expect("source plugin manifest should be written");
    std::fs::write(
        source_directory.join("assets").join("readme.txt"),
        "package asset",
    )
    .expect("source plugin asset should be written");

    let state = store
        .import_plugin_directory_path(&source_directory)
        .expect("plugin directory should install");

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("installed plugin should be listed");
    let install_path = plugin
        .install_path
        .as_ref()
        .expect("installed plugin should expose install path");
    let installed_directory = PathBuf::from(install_path);

    assert_eq!(plugin.package_kind, "directory");
    assert!(installed_directory.join("manifest.json").exists());
    assert!(
        installed_directory
            .join("assets")
            .join("readme.txt")
            .exists()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn installs_opplugin_package_and_extracts_assets() {
    let (mut store, directory) = temp_store();
    let package_path = directory.join("subtitle-styler.opplugin");
    write_opplugin_package(&package_path, subtitle_plugin_json());

    let state = store
        .import_plugin_package_path(&package_path)
        .expect("OpenPlayer plugin package should install");

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("installed plugin should be listed");
    let install_path = plugin
        .install_path
        .as_ref()
        .expect("installed plugin should expose install path");
    let installed_directory = PathBuf::from(install_path);

    assert_eq!(plugin.package_kind, "opplugin");
    assert!(installed_directory.join("manifest.json").exists());
    assert!(
        installed_directory
            .join("assets")
            .join("readme.txt")
            .exists()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn imports_capability_plugin_without_theme_and_lists_settings() {
    let (mut store, directory) = temp_store();

    let state = store
        .import_theme_plugin_json(subtitle_plugin_json())
        .expect("capability plugin manifest should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .expect("subtitle plugin should be listed");
    assert!(plugin.enabled);
    assert_eq!(plugin.theme_count, 0);
    assert_eq!(plugin.capability_count, 1);
    assert_eq!(plugin.setting_count, 2);
    assert_eq!(plugin.permissions, vec!["mpv.subtitleStyle"]);
    assert_eq!(plugin.settings[0].value, serde_json::json!(42));
}

#[test]
fn imports_extended_subtitle_typography_mpv_settings() {
    let (mut store, directory) = temp_store();

    let state = store
        .import_theme_plugin_json(extended_subtitle_plugin_json())
        .expect("extended subtitle typography plugin should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.typography")
        .expect("subtitle typography plugin should be listed");
    let mpv_properties: Vec<&str> = plugin
        .settings
        .iter()
        .filter_map(|setting| setting.mpv_property.as_deref())
        .collect();

    assert_eq!(
        mpv_properties,
        vec!["sub-spacing", "sub-outline-size", "sub-shadow-offset"]
    );
}

#[test]
fn imports_documented_subtitle_typography_example_plugin() {
    let (mut store, directory) = temp_store();
    let manifest = include_str!("../../fixtures/plugins/subtitle-typography/manifest.json");

    let state = store
        .import_theme_plugin_json(manifest)
        .expect("documented subtitle typography example should import");
    let _ = std::fs::remove_dir_all(&directory);

    let plugin = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.typography")
        .expect("example subtitle typography plugin should be listed");

    assert_eq!(plugin.setting_count, 8);
    assert_eq!(plugin.action_count, 0);
    assert!(
        !plugin
            .settings
            .iter()
            .any(|setting| setting.mpv_property.as_deref() == Some("sub-line-spacing"))
    );
    let letter_spacing = plugin
        .settings
        .iter()
        .find(|setting| setting.id == "letter-spacing")
        .expect("letter spacing setting should exist");
    assert_eq!(letter_spacing.max, Some(10.0));
    let font_size = plugin
        .settings
        .iter()
        .find(|setting| setting.id == "font-size")
        .expect("font size setting should exist");
    assert_eq!(
        font_size.label_i18n.get("zh-CN").map(String::as_str),
        Some("字号")
    );
}

#[test]
fn rejects_removed_subtitle_line_spacing_mpv_property() {
    let (mut store, directory) = temp_store();

    let error = store
        .import_theme_plugin_json(
            r##"{
                  "id": "dev.openplayer.subtitle.line-spacing",
                  "name": "Removed Subtitle Line Spacing",
                  "version": "1.0.0",
                  "entry": "manifest",
                  "runtime": { "kind": "manifest" },
                  "contributes": {
                    "settings": [
                      {
                        "id": "line-spacing",
                        "label": "Line Spacing",
                        "kind": "number",
                        "placement": "subtitleSettings",
                        "defaultValue": 0,
                        "min": -10,
                        "max": 10,
                        "step": 1,
                        "mpvProperty": "sub-line-spacing"
                      }
                    ]
                  }
                }"##,
        )
        .expect_err("removed subtitle line spacing property should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("unsupported plugin mpv property: sub-line-spacing"));
}

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
fn uninstalling_plugin_removes_state_settings_and_installed_files() {
    let (mut store, directory) = temp_store();
    let source_manifest = directory.join("subtitle-plugin.json");
    std::fs::write(&source_manifest, subtitle_plugin_json())
        .expect("source plugin manifest should be written");
    let installed = store
        .import_plugin_manifest_path(&source_manifest)
        .expect("plugin manifest file should install");
    let install_path = installed
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| plugin.install_path.clone())
        .expect("installed plugin should expose install path");
    store
        .set_plugin_setting(
            "dev.openplayer.subtitle.styler",
            "font-size",
            serde_json::json!(56),
        )
        .expect("valid plugin setting should persist");

    let state = store
        .uninstall_plugin("dev.openplayer.subtitle.styler")
        .expect("plugin should uninstall");
    assert!(
        !state
            .plugins
            .iter()
            .any(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
    );
    assert!(!PathBuf::from(&install_path).exists());

    let reinstalled = store
        .import_plugin_manifest_path(&source_manifest)
        .expect("plugin manifest file should reinstall");
    let font_size = reinstalled
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| {
            plugin
                .settings
                .iter()
                .find(|setting| setting.id == "font-size")
        })
        .map(|setting| setting.value.clone());
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(font_size, Some(serde_json::json!(42)));
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

#[test]
fn persists_valid_plugin_setting_values() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(subtitle_plugin_json())
        .expect("capability plugin manifest should import");

    let state = store
        .set_plugin_setting(
            "dev.openplayer.subtitle.styler",
            "font-size",
            serde_json::json!(56),
        )
        .expect("valid plugin setting should persist");
    drop(store);

    let reopened = AppearanceStore::open(directory.join("settings.redb"))
        .expect("appearance store should reopen");
    let reopened_state = reopened
        .state()
        .expect("appearance state should be readable");
    let _ = std::fs::remove_dir_all(&directory);

    let state_value = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| {
            plugin
                .settings
                .iter()
                .find(|setting| setting.id == "font-size")
        })
        .map(|setting| setting.value.clone());
    let reopened_value = reopened_state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| {
            plugin
                .settings
                .iter()
                .find(|setting| setting.id == "font-size")
        })
        .map(|setting| setting.value.clone());

    assert_eq!(state_value, Some(serde_json::json!(56)));
    assert_eq!(reopened_value, Some(serde_json::json!(56)));
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
fn rejects_plugin_setting_values_outside_schema() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(subtitle_plugin_json())
        .expect("capability plugin manifest should import");

    let error = store
        .set_plugin_setting(
            "dev.openplayer.subtitle.styler",
            "font-size",
            serde_json::json!(120),
        )
        .expect_err("out-of-range plugin setting should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("font-size"));
}

#[test]
fn falls_back_to_default_when_stored_plugin_setting_no_longer_matches_schema() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(subtitle_plugin_json())
        .expect("capability plugin manifest should import");
    store
        .set_plugin_setting(
            "dev.openplayer.subtitle.styler",
            "font-size",
            serde_json::json!(56),
        )
        .expect("valid plugin setting should persist");
    let updated_manifest = subtitle_plugin_json().replace("\"max\": 96", "\"max\": 48");
    let state = store
        .import_theme_plugin_json(&updated_manifest)
        .expect("plugin update should import with stricter schema");
    let _ = std::fs::remove_dir_all(&directory);

    let value = state
        .plugins
        .iter()
        .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        .and_then(|plugin| {
            plugin
                .settings
                .iter()
                .find(|setting| setting.id == "font-size")
        })
        .map(|setting| setting.value.clone());

    assert_eq!(value, Some(serde_json::json!(42)));
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
    assert!(is_supported_plugin_permission("network.request"));
    assert!(is_supported_plugin_permission("filesystem.pick"));
    assert!(is_supported_plugin_permission("filesystem.reveal"));
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

#[test]
fn disabling_active_plugin_theme_falls_back_to_studio_dark() {
    let (mut store, directory) = temp_store();
    store
        .import_theme_plugin_json(ocean_plugin_json())
        .expect("theme plugin manifest should import");
    store
        .set_theme("dev.openplayer.theme.ocean.dark")
        .expect("plugin theme should be selectable");

    let state = store
        .set_plugin_enabled("dev.openplayer.theme.ocean", false)
        .expect("theme plugin should be disabled");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(state.active_theme_id, "studio-dark");
    assert!(
        state
            .themes
            .iter()
            .any(|theme| theme.id == "dev.openplayer.theme.ocean.dark" && !theme.enabled)
    );
}

#[test]
fn rejects_invalid_theme_plugin_color() {
    let (mut store, directory) = temp_store();
    let invalid = ocean_plugin_json().replace("\"#62c7b7\"", "\"blue\"");

    let error = store
        .import_theme_plugin_json(&invalid)
        .expect_err("invalid color token should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert!(error.contains("accent"));
}

#[test]
fn player_preferences_default_false_and_persist() {
    let (mut store, directory) = temp_store();

    assert_eq!(
        store.preferences().expect("preferences should be readable"),
        PlayerPreferences {
            incognito_mode: false,
            quiet_keyboard_controls: false,
            language_mode: "system".to_string(),
        }
    );

    store
        .set_bool_preference(INCOGNITO_MODE_KEY, true)
        .expect("incognito mode should be persisted");
    store
        .set_bool_preference(QUIET_KEYBOARD_CONTROLS_KEY, true)
        .expect("quiet keyboard controls should be persisted");
    let preferences = store
        .set_language_mode("en-US")
        .expect("language mode should be persisted");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(
        preferences,
        PlayerPreferences {
            incognito_mode: true,
            quiet_keyboard_controls: true,
            language_mode: "en-US".to_string(),
        }
    );
}

#[test]
fn rejects_invalid_language_mode_preference() {
    let (mut store, directory) = temp_store();

    let error = store
        .set_language_mode("fr-FR")
        .expect_err("unsupported language modes should be rejected");
    let _ = std::fs::remove_dir_all(&directory);

    assert_eq!(error, "invalid language mode");
}
