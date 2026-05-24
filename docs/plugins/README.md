# OpenPlayer Plugins

OpenPlayer plugins are currently **package-based extensions**. A user-facing
plugin ships as a `.opplugin` file, which is a zip archive with a root
`manifest.json`. The manifest can contribute themes, capability metadata,
validated settings, UI-slot controls, and an optional sandboxed JavaScript worker
runtime.

The manifest format is intentionally forward-compatible with future sandboxed
third-party code runtimes. `webviewJs` is supported through an isolated Web
Worker bridge. `wasm` remains reserved until a separate WASM sandbox is
implemented.

## Official Plugins

Official plugin source code and release packages live in the separate
[openplayer-plugins](https://github.com/AreChen/openplayer-plugins) repository.
This player repository owns the host API, manifest behavior, and security
model. Keep official plugin implementations in `openplayer-plugins`; keep only
small host test fixtures in this repository when integration tests need a
manifest.

## Current Capabilities

- Install `.opplugin` packages from the Plugins settings page.
- Drag `.opplugin` packages into the player window to install them.
- Import a plugin folder or raw JSON manifest for local development.
- Uninstall plugins and remove their managed files, settings, themes, and
  enablement state.
- Enable or disable imported plugins.
- Persist plugin manifests and plugin setting values in redb.
- Render plugin settings in the central Plugins settings page.
- Render settings assigned to the subtitle/track panel in that panel.
- Render plugin actions in UI slots such as the control strip, context menu, and
  playlist actions.
- Apply whitelisted subtitle-related mpv properties from plugin settings:
  `sub-font`, `sub-font-size`, `sub-scale`, `sub-pos`, `sub-color`,
  `sub-spacing`, `sub-outline-size`, `sub-border-size`, and
  `sub-shadow-offset`.
- Let declarative actions call safe built-in capability APIs:
  `player.captureScreenshot`, `player.openStream`, and
  `player.openStreamDialog`.
- Let capture plugins start and stop lightweight mpv native stream recording
  through permissioned host commands.
- Execute optional `webviewJs` runtime scripts in a Web Worker sandbox with no
  DOM, Tauri API, local filesystem access, or direct host privileges.

## Package Format

`.opplugin` is a zip archive:

```text
subtitle-styler.opplugin
├── manifest.json
└── assets/
    └── ...
```

Rules:

- `manifest.json` must exist at the archive root.
- All archive entries must be relative package paths.
- Path traversal and symlinks are rejected.
- Packages are extracted to OpenPlayer's managed app-data plugin directory.
- The current package limit is 1024 entries and 128 MiB uncompressed.

During development, a directory with the same layout can be imported directly
from the Plugins settings page.

## Manifest Example

```json
{
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
        "labelI18n": {
          "zh-CN": "字号"
        },
        "descriptionI18n": {
          "zh-CN": "调整字幕基础字号。"
        },
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
      },
      {
        "id": "screenshot",
        "label": "Screenshot",
        "placement": "controls.right",
        "command": "player.captureScreenshot",
        "icon": "camera",
        "requiresMedia": true,
        "args": {
          "openFolder": true
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
      }
    ]
  }
}
```

Theme plugins remain supported:

```json
{
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
}
```

## Runtime

Current supported runtime:

- `manifest` - declarative only, no third-party code execution.
- `webviewJs` - executes the package entry script in an isolated Web Worker.

Reserved runtime kinds:

- `wasm` - future sandboxed WebAssembly runtime.

`webviewJs` requires:

```json
"runtime": {
  "kind": "webviewJs",
  "entry": "dist/plugin.js",
  "sandbox": "openplayer-worker"
}
```

The runtime entry must be a relative path inside the installed plugin package.
The current script size limit is 1 MiB.

Inside the worker, plugins use the injected `openplayer` bridge:

```js
openplayer.onReady(async () => {
  await openplayer.request("player.captureScreenshot", { openFolder: false });
});
```

Supported worker bridge requests match the built-in action command allowlist.
Permissioned requests such as `player.captureScreenshot` and `player.openStream`
are rejected unless the plugin manifest declares the required permission.
`player.openStreamDialog` is available to declarative actions and opens the
host-owned network stream dialog.
Capture requests such as `player.startRecording`, `player.stopRecording`,
`player.toggleRecording`, and `player.recordingState` require `mpv.capture`.

## Contributions

`contributes.themes` adds theme tokens.

`contributes.capabilities` declares what the plugin wants to enhance. Supported
capability kinds:

- `subtitleStyle`
- `capture`
- `streamSource`
- `aiTranscription`
- `aiTranslation`

Supported permission declarations:

- `mpv.subtitleStyle`
- `mpv.capture`
- `media.openStream`
- `network.request`
- `ai.transcribe`
- `ai.translate`

`contributes.settings` adds validated controls. Supported setting kinds:

- `boolean`
- `number`
- `text`
- `select`
- `color`
- `directory`

Setting, action, and capability display text may include optional locale maps
next to their default English strings. OpenPlayer currently resolves `zh-CN`,
language-only fallbacks such as `zh`, then `en-US`/`en` before using the default:

```json
{
  "label": "Font Size",
  "description": "Base subtitle font size.",
  "labelI18n": {
    "zh-CN": "字号"
  },
  "descriptionI18n": {
    "zh-CN": "调整字幕基础字号。"
  }
}
```

Supported UI placements:

- `pluginSettings`
- `subtitleSettings`
- `captureSettings`
- `streamSettings`
- `controls.left`
- `controls.center`
- `controls.right`
- `contextMenu`
- `overlay.status`
- `playlist.actions`

The current UI renders `pluginSettings` centrally and renders
`subtitleSettings` inside the track/subtitle panel. Other placements are accepted
as stable manifest slots for future UI rendering.

`contributes.actions` adds declarative buttons or menu items. Actions do not run
plugin code; they invoke a small allowlist of built-in OpenPlayer commands.

Supported action placements:

- `controls.left`
- `controls.center`
- `controls.right`
- `contextMenu`
- `overlay.status`
- `playlist.actions`

The current UI renders action placements for `controls.left`, `controls.center`,
`controls.right`, `contextMenu`, and `playlist.actions`.

Supported action commands:

- `player.openMedia`
- `player.openStream`
- `player.openStreamDialog`
- `player.captureScreenshot`
- `player.startRecording`
- `player.stopRecording`
- `player.toggleRecording`
- `player.togglePlayback`
- `player.stop`
- `player.restart`
- `player.togglePlaylist`
- `player.toggleTracks`
- `player.toggleLoop`
- `player.toggleSpeed`
- `window.toggleFullscreen`
- `window.toggleAlwaysOnTop`
- `app.openSettings`

Supported action icons:

- `folder`
- `folderAdd`
- `play`
- `pause`
- `stop`
- `restart`
- `list`
- `tracks`
- `settings`
- `fullscreen`
- `pin`
- `plugin`
- `camera`
- `record`
- `stream`
- `info`

Action arguments:

- `player.captureScreenshot` accepts optional `args.format`,
  `args.formatSetting`, `args.directorySetting`, `args.openFolder`, and
  `args.openFolderSetting`.
  Supported screenshot formats are `png`, `jpg`, and `webp`. It requires the
  plugin to declare `mpv.capture`.
- `player.startRecording` and `player.toggleRecording` accept optional
  `args.format`, `args.formatSetting`, `args.directorySetting`,
  `args.openFolder`, and `args.openFolderSetting`. Supported recording containers are `mkv`, `mp4`, and
  `ts`. Local files and HTTP/HTTPS streams are clipped with mpv `dump-cache`;
  live protocols such as RTSP, RTMP, SRT, and UDP use mpv `stream-record`.
  Recording uses the requested container but does not transcode media, so codec
  and muxer compatibility depends on the source. These commands require
  `mpv.capture`.
- `player.stopRecording` accepts optional `args.openFolder` and
  `args.openFolderSetting`. It requires `mpv.capture`.
- `player.openStream` requires `args.url` and accepts optional `args.name`.
  Supported stream protocols are `http`, `https`, `rtmp`, `rtmps`, `rtsp`,
  `rtsps`, `srt`, and `udp`. It requires the plugin to declare
  `media.openStream`.
- `player.openStreamDialog` opens OpenPlayer's network stream dialog with recent
  RTSP, RTMP, and HTTP(S) streams. It requires the plugin to declare
  `media.openStream`.

## Validation Rules

- Plugin IDs must be dotted lowercase identifiers.
- Versions must use `major.minor.patch`.
- A plugin must contribute at least one theme, capability, setting, or action.
- Unknown manifest fields are rejected.
- Setting defaults must match their declared type.
- Locale maps such as `labelI18n` must use short ASCII locale keys and
  non-empty text values.
- Numeric settings must respect `min`, `max`, and `step`.
- Select settings must define non-empty unique options.
- `mpvProperty` is only accepted for `subtitleSettings` and only for the
  whitelisted subtitle properties listed above.
- Actions are rejected unless their placement, command, and icon are on the
  documented allowlists.
- Capability actions are rejected unless their required permissions are declared.
- Stream actions are rejected for local file URLs, whitespace, unsupported
  protocols, or missing URLs.
- `.opplugin` packages are rejected unless they contain a root `manifest.json`
  and all entries stay inside the package.
