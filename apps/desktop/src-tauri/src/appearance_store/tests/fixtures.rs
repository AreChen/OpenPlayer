use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

use super::{AppearanceStore, PLUGIN_MANIFEST_FILE};

static TEMP_STORE_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn temp_store() -> (AppearanceStore, PathBuf) {
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

pub(super) fn ocean_plugin_json() -> &'static str {
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

pub(super) fn subtitle_plugin_json() -> &'static str {
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

pub(super) fn extended_subtitle_plugin_json() -> &'static str {
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

pub(super) fn webview_runtime_plugin_json() -> &'static str {
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

pub(super) fn view_plugin_json() -> &'static str {
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

pub(super) fn wasm_runtime_plugin_json() -> &'static str {
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

pub(super) fn action_plugin_json() -> &'static str {
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

pub(super) fn capability_action_plugin_json() -> &'static str {
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

pub(super) fn write_opplugin_package(path: &Path, manifest_json: &str) {
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
