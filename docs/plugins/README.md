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

For current SDK 1.5 runtime usage, examples, permissions, events, mpv controls,
custom views, and verification commands, read
[`sdk-1.5-developer-guide.md`](./sdk-1.5-developer-guide.md).

## Current Capabilities

- Install `.opplugin` packages from the Plugins settings page.
- Drag `.opplugin` packages into the player window to install them.
- Import a plugin folder or raw JSON manifest for local development.
- Uninstall plugins and remove their managed files, settings, themes, and
  enablement state.
- Enable or disable imported plugins.
- Persist plugin manifests and plugin setting values in redb.
- Render plugin settings in the central Plugins settings page.
- Let plugins create session-local host-managed task snapshots for long-running
  transcription, translation, analysis, and batch work with
  `openplayer.tasks`.
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
- Let advanced plugins use permissioned, allowlisted mpv core controls for
  safe properties, commands, OSD text, script messages, and plugin-scoped video
  filters.
- Execute optional `webviewJs` runtime scripts in a Web Worker sandbox with no
  DOM, Tauri API, local filesystem access, or direct host privileges.
- Send playback and media lifecycle events to runtime plugins.
- Let runtime plugins participate in `media.opening` and return safe mpv
  `loadfile` options such as HLS demuxer hints before playback starts.
- Let runtime and view plugins ask the host for normalized current playback
  windows with `openplayer.media.currentSegment`, then export short managed WAV
  clips from those windows with `openplayer.audio.extractClip`.
- Let runtime and view plugins upload current-plugin managed artifacts through
  `openplayer.network.request({ bodyFile })` without exposing arbitrary local
  filesystem reads.
- Let runtime and view plugins create subtitles from structured `SubtitleCue[]`
  input with `openplayer.subtitle.loadGeneratedCues`, or from standard subtitle
  text with `openplayer.subtitle.loadGenerated`; the host writes results into a
  plugin-scoped managed directory and loads them as mpv subtitle tracks.
- Let plugins list, replace, and remove only their own generated subtitle
  tracks with `openplayer.subtitle.listGenerated`,
  `openplayer.subtitle.replaceGenerated`, and
  `openplayer.subtitle.removeGenerated`.

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
  "apiVersion": "1",
  "minHostVersion": "1.5.1",
  "author": "OpenPlayer Team",
  "updateUrl": "https://github.com/AreChen/openplayer-plugins/releases",
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
  "apiVersion": "1",
  "minHostVersion": "1.5.1",
  "author": "OpenPlayer Team",
  "updateUrl": "https://github.com/AreChen/openplayer-plugins/releases",
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
  "sandbox": "openplayer-worker",
  "events": ["media.loaded", "playback.snapshot"]
}
```

The runtime entry must be a relative path inside the installed plugin package.
The current script size limit is 1 MiB.
`events` is optional, but runtime plugins only receive non-lifecycle event
payloads that they declare here. Plugins can then enable or disable those
declared subscriptions at runtime with `openplayer.events.subscribe()` and
`openplayer.events.unsubscribe()`.

Inside the worker, plugins use the injected `openplayer` bridge:

```js
openplayer.onReady(async () => {
  if (openplayer.capabilities.has("mpv.wall")) {
    console.log(`OpenPlayer ${openplayer.host.version} supports native wall tiles`);
    console.log(`SDK compatibility: ${openplayer.api.compatibility.compatibility}`);
  }

  const launchCount = (await openplayer.storage.get("launch.count")) ?? 0;
  await openplayer.storage.set("launch.count", Number(launchCount) + 1);
  await openplayer.events.subscribe("playback.snapshot");
});

openplayer.onEvent((event, payload) => {
  if (event === "playback.snapshot") {
    console.log(payload.position, payload.duration);
  }
});

openplayer.media.onBeforeOpen((media) => {
  if (/^https?:\/\//i.test(media.path) && /\.m3u8(?:[?#]|$)/i.test(media.path)) {
    return {
      loadOptions: {
        demuxer: "+lavf",
        "demuxer-lavf-format": "hls"
      }
    };
  }
  return null;
});

openplayer.commands.register("plugin.open-network-stream", async () => {
  await openplayer.media.openStreamDialog();
});
```

SDK 1.5.1 exposes `openplayer.host`, `openplayer.api`, and
`openplayer.capabilities` in both worker runtimes and custom views.
`host.version` identifies the running OpenPlayer build.
`api.compatibility` gives plugins a stable compatibility block for feature
gating. `capabilities.list()` reports host-supported API families, while
`capabilities.permissions()` reports the permissions granted to the current
plugin from its manifest.

Supported worker bridge requests match the built-in action command allowlist.
Permissioned requests such as `player.captureScreenshot` and `player.openStream`
are rejected unless the plugin manifest declares the required permission.
Low-risk requests include `plugin.getSettings`, `plugin.storage.*`,
`player.currentMedia`, `player.snapshot`, `player.play`, `player.pause`,
`player.seek`, `player.frameStep`, `player.frameBackStep`, `player.setVolume`,
`player.setSpeed`, `player.setLoopMode`, `player.setVideoFill`,
`player.setSubtitleDelay`, `player.selectTrack`, `ui.*`, `playlist.current`,
`playlist.playIndex`, and `playlist.clear`.
`network.request` requires `network.request`. `filesystem.pickMedia`,
`filesystem.pickDirectory`, `playlist.openMediaFiles`,
`playlist.appendMediaFiles`, and `subtitle.pickExternal` require
`filesystem.pick`. `filesystem.revealPath` and `filesystem.openDirectory`
require `filesystem.reveal`.
`player.wall.open`, `player.wall.layout`, `player.wall.snapshot`,
`player.wall.setVisible`, and `player.wall.close` require `mpv.wall` and are
intended for native multi-stream views that need protocols the WebView cannot
decode directly, such as RTSP and RTMP. `layout` accepts viewport-relative tile
rectangles and is safe to call from debounced or throttled resize handlers.
`setVisible(false)` temporarily hides the native wall so plugin-owned dialogs can
receive pointer input above native video child windows.
Wall snapshots report native buffer and bitrate telemetry, plus
`transportLatencyMs`/`transportLatencySource` when the host can bind the
displayed frame to transport timing.
`player.wall.open` accepts optional per-tile `playback` tuning for native RTSP
tiles. Runtime plugins can choose `latencyMode` (`off`, `stable`, `balanced`,
or `aggressive`), `rtspTransport` (`tcp` or `udp`), and a bounded `bufferMs`
target; the host validates these values before applying mpv options.
WebRTC/WHEP plugins should render browser video tiles inside their custom view
and use `network.request` for WHEP HTTP signaling when they need host-mediated
requests.
`openplayer.mpv` exposes a broader but still allowlisted mpv core surface.
`getProperty`, `setProperty`, and generic safe commands require `mpv.core`.
`showText` requires `mpv.osd`; `scriptMessage` requires `mpv.scriptMessage`;
`filters.add`, `filters.remove`, `audioFilters.add`, and
`audioFilters.remove` require `mpv.filters`. `setAbLoop` and `clearAbLoop`
require `mpv.core`. The backend validates property names, value types, numeric
ranges, allowed commands, and plugin-owned filter labels before sending
anything to libmpv. This API intentionally does not expose raw `loadfile`,
shell-like commands, arbitrary filter graphs, or unsafe process/configuration
properties.
`openplayer.plugin.getSettings()` returns the current values declared in
`contributes.settings`. `plugin.storage.get`, `plugin.storage.set`,
`plugin.storage.remove`, and `plugin.storage.list` provide redb-backed
plugin-private JSON storage. Storage keys are namespaced by plugin ID and
removed when the plugin is uninstalled.
`player.openStreamDialog` is available to declarative actions and opens the
host-owned network stream dialog.
Capture requests such as `player.startRecording`, `player.stopRecording`,
`player.toggleRecording`, and `player.recordingState` require `mpv.capture`.
Changing `path` from `onBeforeOpenMedia` requires `media.openStream`.
Returning `loadOptions` from `onBeforeOpenMedia` requires `mpv.loadOptions`.
Only `demuxer` and `demuxer-lavf-format` are accepted at this stage.

Declarative actions may also use plugin-owned commands with the `plugin.*`
prefix, such as `plugin.open-network-stream`. Those actions are routed to the
same plugin's `webviewJs` runtime through `openplayer.registerCommand`.
Runtime commands can open plugin-owned custom views with
`openplayer.ui.openView("view-id")`. Custom views are static HTML files inside
the installed plugin package and receive a smaller `openplayer` bridge for
settings, plugin storage, host-mediated HTTP(S) requests, toasts, and closing
the view. OpenPlayer injects a restrictive Content Security Policy into custom
views, including `connect-src 'none'`, so HTTP(S) requests should go through
`openplayer.network.request` instead of direct browser network APIs. View
bridges also expose `openplayer.player.wall` and `openplayer.mpv` for plugins
that render a custom control surface above native mpv wall tiles.

For authoring TypeScript plugins, use the `@openplayer/plugin-sdk` types from
the official `openplayer-plugins` repository. Runtime plugins do not need to
bundle the SDK; OpenPlayer injects the `openplayer` global.

## Contributions

`contributes.themes` adds theme tokens.

`contributes.capabilities` declares what the plugin wants to enhance. Supported
capability kinds:

- `subtitleStyle`
- `capture`
- `streamSource`
- `audioTool`
- `subtitleTool`
- `mpvControl`

Supported permission declarations:

- `mpv.subtitleStyle`
- `mpv.loadOptions`
- `mpv.capture`
- `mpv.wall`
- `mpv.core`
- `mpv.filters`
- `mpv.osd`
- `mpv.scriptMessage`
- `media.openStream`
- `filesystem.pick`
- `filesystem.reveal`
- `network.request`
- `audio.extract`
- `subtitle.write`

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

`contributes.actions` adds declarative buttons or menu items. Actions can invoke
a small allowlist of built-in OpenPlayer commands, or a plugin-owned
`plugin.*` command registered by the same `webviewJs` runtime.

`contributes.views` adds static HTML plugin views:

```json
{
  "id": "wall",
  "title": "Multi-Stream Wall",
  "titleI18n": {
    "zh-CN": "多路流媒体墙"
  },
  "presentation": "sidePanel",
  "frameOpacitySetting": "panel-opacity",
  "entry": "view/index.html"
}
```

The entry must be a safe relative package path. OpenPlayer reads the HTML from
the installed plugin package and mounts it in an iframe with an injected bridge.
The default presentation is `overlay`; use `sidePanel` for right-side panels.
`frameOpacitySetting` is optional and must reference a `number` setting in
`contributes.settings`; OpenPlayer applies that setting as host-level iframe
opacity for user-tunable translucent side panels.

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
- `plugin.*`

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
- `tv`
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
- `plugin.*` commands are routed to `openplayer.registerCommand` in the same
  plugin runtime. The action `args` object is passed through unchanged after a
  small size check.

## Validation Rules

- Plugin IDs must be dotted lowercase identifiers.
- Versions must use `major.minor.patch`.
- `apiVersion` defaults to `1`; unknown plugin API versions are rejected.
- `minHostVersion`, when present, must use `major.minor.patch` and cannot be
  newer than the running OpenPlayer version.
- `author`, when present, must be non-empty.
- `updateUrl`, when present, must be an HTTP(S) URL.
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
