# OpenPlayer Plugin SDK 1.5 Developer Guide

This guide is the current working reference for developers and AI agents that
build OpenPlayer plugins against SDK 1.5.

OpenPlayer plugins are package-based extensions. A plugin package contributes a
validated `manifest.json`, optional settings/actions/views, and optionally a
`webviewJs` runtime script that runs in an isolated worker. The host owns mpv,
native dialogs, persistent storage, security validation, and UI placement.

## Where The SDK Lives

- Host bridge constants:
  `apps/desktop/src/app/pluginRuntime/constants.ts`
- Worker bridge source:
  `apps/desktop/src/app/pluginRuntime/workerSource/`
- Custom view bridge:
  `apps/desktop/src/app/pluginRuntime/viewDocument.ts`
- Runtime command handlers:
  `apps/desktop/src/hooks/pluginRuntimeCommands/`
- Manifest validation and runtime source loading:
  `apps/desktop/src-tauri/src/appearance_store/manifest/` and
  `apps/desktop/src-tauri/src/appearance_store/runtime/`
- Permissioned mpv backend:
  `apps/desktop/src-tauri/src/mpv_embed/plugin_core.rs` and
  `apps/desktop/src-tauri/src/mpv_embed/commands/playback/plugins.rs`
- Public TypeScript SDK package:
  `openplayer-plugins/packages/sdk/index.d.ts`
- Official plugin examples:
  `openplayer-plugins/plugins/` and `openplayer-plugins/templates/`

When the SDK surface changes, update the host bridge, backend allowlists,
public TypeScript types, official plugin examples, and docs together.

## Minimal Runtime Plugin

```json
{
  "id": "dev.example.openplayer.plugin",
  "name": "Example Plugin",
  "version": "1.5.0",
  "apiVersion": "1",
  "minHostVersion": "1.5.0",
  "author": "Example Author",
  "updateUrl": "https://github.com/example/openplayer-plugin/releases",
  "description": "Example runtime plugin.",
  "entry": "manifest",
  "runtime": {
    "kind": "webviewJs",
    "entry": "runtime/plugin.js",
    "sandbox": "openplayer-worker",
    "events": ["media.loaded", "playback.snapshot"]
  },
  "contributes": {
    "capabilities": [
      {
        "id": "example-stream",
        "name": "Example Stream",
        "kind": "streamSource",
        "permissions": ["media.openStream", "mpv.loadOptions"]
      }
    ],
    "actions": [
      {
        "id": "open-example-stream",
        "label": "Open Stream",
        "placement": "contextMenu",
        "command": "plugin.open-example-stream",
        "icon": "stream"
      }
    ]
  }
}
```

```js
/// <reference path="../../packages/sdk/index.d.ts" />

"use strict";

openplayer.onReady(async () => {
  if (!openplayer.capabilities.has("media.openStream")) {
    await openplayer.ui.toast(
      `OpenPlayer ${openplayer.api.compatibility.hostVersion} cannot open plugin streams yet.`,
    );
    return;
  }

  await openplayer.events.subscribe("playback.snapshot");
});

openplayer.onEvent((event, payload) => {
  if (event === "playback.snapshot") {
    console.log(payload.position, payload.duration);
  }
});

openplayer.commands.register("plugin.open-example-stream", async () => {
  await openplayer.media.openStreamDialog();
});
```

## Runtime Rules

- `webviewJs` scripts run in an isolated worker. They do not get DOM, Tauri,
  filesystem, direct network, or direct mpv access.
- Custom views run in an iframe with a restrictive injected CSP. They can render
  UI, but host access still goes through `window.openplayer`.
- Runtime scripts and custom views should feature-detect with
  `openplayer.capabilities.has(...)` and read
  `openplayer.api.compatibility` before relying on optional features.
- Plugin-owned commands must use the `plugin.*` prefix. The host routes them
  only to the runtime for the same plugin that declared the action.
- Runtime event delivery is opt-in. Declare allowed event names in
  `runtime.events`, then subscribe with `openplayer.events.subscribe(event)`.

## Permission Model

Low-risk reads, plugin-private storage, toasts, and some UI helpers are
available without extra permissions. Anything that opens media, touches native
files, makes network requests, records/captures, or controls mpv needs an
explicit manifest permission.

Important permissions:

- `media.openStream`: open network streams and allow `media.opening` hooks to
  rewrite stream paths.
- `mpv.loadOptions`: return safe mpv `loadfile` options from media-opening
  hooks, such as HLS demuxer hints.
- `mpv.capture`: screenshots and native recording.
- `mpv.wall`: native multi-stream wall tiles.
- `mpv.core`: allowlisted mpv properties, safe commands, and AB loop helpers.
- `mpv.filters`: plugin-scoped video/audio filters.
- `mpv.osd`: temporary mpv OSD text.
- `mpv.scriptMessage`: allowlisted script messages.
- `network.request`: validated HTTP(S) requests.
- `filesystem.pick`: user-mediated file or directory picking.
- `filesystem.reveal`: reveal/open local paths in the system file manager.

Do not document or implement plugins that bypass these permissions with raw
Tauri calls, raw filesystem access, raw sockets, arbitrary mpv commands, or
unvalidated filter graphs.

## Current SDK Surface

### Metadata And Capability Detection

```js
openplayer.sdkVersion;
openplayer.host.version;
openplayer.api.compatibility;
openplayer.capabilities.list();
openplayer.capabilities.has("mpv.core");
openplayer.capabilities.permissions();
openplayer.capabilities.hasPermission("mpv.core");
```

Use this before enabling optional UI or mpv features.

### Events

```js
await openplayer.events.subscribe("media.loaded");
await openplayer.events.unsubscribe("media.loaded");
const declared = openplayer.events.subscribed();
const supported = await openplayer.events.list();

openplayer.onEvent((event, payload) => {
  // event is delivered only when declared and subscribed.
});
```

Supported events in SDK 1.5 include:

- `app.ready`
- `media.opening`
- `media.loaded`
- `playback.snapshot`
- `playback.started`
- `playback.paused`
- `playback.ended`
- `playback.stopped`
- `playback.seeked`
- `playback.volumeChanged`
- `playback.speedChanged`
- `tracks.changed`
- `theme.changed`
- `window.fullscreenChanged`
- `plugin.view.opened`
- `plugin.view.closed`

### Media And Playlist

```js
openplayer.media.openStream(url, {
  name: "Camera 1",
  loadOptions: { demuxer: "+lavf", "demuxer-lavf-format": "hls" },
});
openplayer.media.openStreamDialog();
openplayer.media.current();
openplayer.media.snapshot();
openplayer.playlist.current();
openplayer.playlist.playIndex(0);
openplayer.playlist.clear();
```

`openplayer.media.onBeforeOpen(handler)` can return a new path, display name,
or safe load options. Path rewrites require `media.openStream`; load options
require `mpv.loadOptions`.

### Player Controls

```js
openplayer.player.play();
openplayer.player.pause();
openplayer.player.togglePlayback();
openplayer.player.stop();
openplayer.player.seek({ delta: 5 });
openplayer.player.setVolume(80);
openplayer.player.setSpeed(1.25);
openplayer.player.setLoopMode("one");
openplayer.player.selectTrack("subtitle", 2);
```

Commands that change mpv playback should return or apply host snapshots. Plugin
code should not guess playback state.

### Permissioned mpv Controls

```js
await openplayer.mpv.setProperty("volume", 80);
await openplayer.mpv.command("chapter-next");
await openplayer.mpv.showText("Ready", { durationMs: 1500 });
await openplayer.mpv.scriptMessage("openplayer-plugin", "refresh");
await openplayer.mpv.filters.add("tone", "eq", { brightness: 5 });
await openplayer.mpv.audioFilters.add("gain", "volume", { gainDb: 3 });
await openplayer.mpv.setAbLoop(12.5, 18.0);
await openplayer.mpv.clearAbLoop();
```

The host allowlist intentionally rejects raw `loadfile`, shell-like commands,
unsafe process properties, arbitrary filter graphs, and plugin labels outside
the current plugin namespace.

### Native Multi-Stream Wall

```js
await openplayer.player.wall.open([
  {
    id: "cam-1",
    url: "rtsp://camera.local/live",
    title: "Camera 1",
    x: 0,
    y: 0,
    width: 1,
    height: 1,
    muted: true,
    playback: {
      latencyMode: "balanced",
      rtspTransport: "tcp",
      bufferMs: 600
    }
  }
]);

await openplayer.player.wall.layout([{ id: "cam-1", x: 0, y: 0, width: 1, height: 1 }]);
const snapshots = await openplayer.player.wall.snapshot();
await openplayer.player.wall.setVisible(false);
await openplayer.player.wall.close();
```

Use the native wall for protocols the WebView cannot decode directly, such as
RTSP and RTMP. WebRTC/WHEP views should render browser `<video>` tiles and use
`openplayer.network.request` for signaling.

### Storage, Network, Filesystem, UI

```js
await openplayer.storage.set("state", { enabled: true });
const state = await openplayer.storage.get("state");

const response = await openplayer.network.request({
  url: "https://example.com/whep",
  method: "POST",
  headers: { "Content-Type": "application/sdp" },
  body: offerSdp,
  timeoutMs: 8000,
});

const files = await openplayer.filesystem.pickMedia({ multiple: true });
const directory = await openplayer.filesystem.pickDirectory();

await openplayer.ui.toast("Plugin ready");
await openplayer.ui.openSettings("plugins");
await openplayer.ui.openPanel("playlist");
await openplayer.ui.openView("wall");
await openplayer.ui.closeView();
```

Storage is plugin-private, redb-backed, and removed when the plugin is
uninstalled.

## Custom Views

Custom views are best for rich plugin-owned UI. Keep these rules:

- Use host theme tokens injected as CSS variables, such as `--op-accent`,
  `--op-panel`, `--op-text`, and `--op-danger`.
- Route all privileged actions through `openplayer`; do not use direct fetch
  for host-mediated network operations unless the feature is intentionally
  browser-local.
- When a view overlays native mpv wall tiles, call
  `openplayer.player.wall.setVisible(false)` before opening plugin-owned modal
  UI and restore visibility after closing it.
- Close native resources in `beforeunload` and when the view closes.

## Official Plugin Improvement Checklist

When updating official plugins for SDK 1.5:

1. Keep manifest `version`, `apiVersion`, and `minHostVersion` aligned with the
   host release.
2. Use `openplayer.api.compatibility` and `openplayer.capabilities.has(...)`
   before invoking optional 1.5 features.
3. Declare runtime events only when the plugin actually consumes them.
4. Prefer high-level APIs (`media`, `player`, `playlist`, `ui`) before using
   permissioned `mpv` controls.
5. Request the smallest permission set that covers the behavior.
6. Put reusable UI logic in plugin-local scripts and package them through
   `openplayer-plugins`.
7. Add or update tests in `openplayer-plugins/tests/` when the public SDK
   contract changes.

## Verification

For host SDK changes in this repository:

```powershell
cd apps/desktop
npm run verify:shell
npm run build
cd ..\..
cargo test -p openplayer-desktop
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

For official plugin changes in `openplayer-plugins`:

```powershell
npm test
npm run build
git diff --check
```
