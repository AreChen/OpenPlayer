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
  "version": "1.5.1",
  "apiVersion": "1",
  "minHostVersion": "1.5.1",
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
- `mpv.capture`: screenshots, native recording, and managed current-frame image
  artifacts.
- `mpv.wall`: native multi-stream wall tiles.
- `mpv.core`: allowlisted mpv properties, safe commands, and AB loop helpers.
- `mpv.filters`: plugin-scoped video/audio filters.
- `mpv.osd`: temporary mpv OSD text.
- `mpv.scriptMessage`: allowlisted script messages.
- `network.request`: validated HTTP(S) requests.
- `filesystem.pick`: user-mediated file or directory picking.
- `filesystem.reveal`: reveal/open local paths in the system file manager.
- `audio.extract`: export short managed WAV clips from the currently loaded
  media for transcription, analysis, or media-understanding plugins.
- `subtitle.read`: read only the currently displayed subtitle cue from the
  selected subtitle track.
- `subtitle.write`: create and load plugin-generated subtitle tracks for the
  currently loaded media.

There is no current provider-specific AI permission. Transcription,
translation, subtitle cleanup, OCR subtitle extraction, and media-understanding
plugins should compose the generic media segment, audio, capture, network,
subtitle, player, task, and storage APIs instead of asking the core for one-off
provider support.

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

### Plugin Tasks

Use `openplayer.tasks` for long-running plugin work such as transcription,
translation, subtitle cleanup, OCR subtitle extraction, media analysis, or batch
playlist operations. Task state is host-managed and scoped to the current
plugin. It is session-local runtime state, not persistent plugin storage.

```js
const task = await openplayer.tasks.start({
  title: "Transcribing current media",
  detail: "Extracting the next audio segment",
  progress: 0,
  cancellable: true,
  metadata: { mediaPath: "/media/movie.mkv" },
});

for (let segment = 0; segment < segments.length; segment += 1) {
  const latest = await openplayer.tasks.update(task.id, {
    detail: `Transcribing segment ${segment + 1} / ${segments.length}`,
    progress: segment / segments.length,
  });
  if (latest.status === "cancelRequested") {
    await openplayer.tasks.markCancelled(task.id);
    return;
  }
}

await openplayer.tasks.complete(task.id, { subtitleTrack: generated.path });
```

`cancel(taskId)` requests cooperative cancellation and changes a running task to
`cancelRequested` only when the task was created with `cancellable: true`.
Plugins should poll the returned task snapshot from `update` or `list`, stop
their own work, and then call `markCancelled(taskId)`.

### Media And Playlist

```js
openplayer.media.openStream(url, {
  name: "Camera 1",
  loadOptions: { demuxer: "+lavf", "demuxer-lavf-format": "hls" },
});
openplayer.media.openStreamDialog();
openplayer.media.current();
openplayer.media.currentSegment({ before: 2, duration: 8 });
openplayer.media.snapshot();
openplayer.playlist.current();
openplayer.playlist.playIndex(0);
openplayer.playlist.clear();
```

`openplayer.media.onBeforeOpen(handler)` can return a new path, display name,
or safe load options. Path rewrites require `media.openStream`; load options
require `mpv.loadOptions`.

`openplayer.media.currentSegment()` returns a host-normalized window around the
current playback position. Use `before` for already-played audio, `duration` for
bounded chunk size, and pass `segment.clip` into `openplayer.audio.extractClip`.
The host clamps segment boundaries to the loaded media duration, so transcription
plugins do not each need to reimplement time-window math.

### Audio Clips

```js
const segment = await openplayer.media.currentSegment({ before: 2, duration: 8 });
const clip = await openplayer.audio.extractClip({
  ...segment.clip,
  sampleRate: 16000,
  channels: "mono",
  includeBase64: true,
});
```

`openplayer.audio.extractClip` exports a short WAV clip from the current media
with a separate mpv instance, so it does not interrupt playback. It requires
`audio.extract`. Use small chunks when requesting `includeBase64`; larger
provider uploads should stay host-mediated instead of giving plugins raw
filesystem access.

### Frame Capture Artifacts

```js
const frame = await openplayer.capture.frame({
  format: "webp",
  includeBase64: false,
});
```

`openplayer.capture.frame` captures the currently displayed video frame into a
plugin-scoped managed image artifact. It requires `mpv.capture` and is meant for
OCR, visual understanding, video summaries, scene tagging, and similar plugins.
Use `includeBase64` only for small inline requests; larger provider uploads
should pass the returned artifact path to `openplayer.network.request`.

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

### Subtitle Generation

Generated subtitles, audio clips, and frame captures are composable SDK
primitives. Use them for AI transcription, translation, OCR subtitle extraction,
subtitle cleanup, and other plugins that produce standard subtitle text. Request
`audio.extract` when the plugin needs a short WAV clip from the current media,
request `mpv.capture` when it needs a current-frame image artifact, request
`subtitle.read` when it reads the current displayed subtitle cue, request
`subtitle.write` when it creates a subtitle track, and request `network.request`
only when it calls an external provider.

```js
const segment = await openplayer.media.currentSegment({ before: 2, duration: 8 });
const clip = await openplayer.audio.extractClip({
  ...segment.clip,
  sampleRate: 16000,
  channels: "mono",
  includeBase64: true,
});

const response = await openplayer.network.request({
  url: "https://example.com/transcribe",
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ audioBase64: clip.bodyBase64 }),
  timeoutMs: 30000,
});

await openplayer.subtitle.loadGeneratedCues({
  name: "AI Transcript",
  format: "vtt",
  cues: JSON.parse(response.text).segments.map((segment) => ({
    start: segment.start,
    end: segment.end,
    text: segment.text,
  })),
  select: true,
});
```

For subtitle translation or cleanup, read the selected track's current cue,
send only that text through a provider, then write a generated cue track. The
host does not expose arbitrary subtitle file reads:

```js
const cue = await openplayer.subtitle.currentCue();
if (cue && cue.start !== null && cue.end !== null) {
  const response = await openplayer.network.request({
    url: "https://example.com/translate",
    method: "POST",
    body: JSON.stringify({ text: cue.text, targetLanguage: "zh-CN" }),
    timeoutMs: 30000,
  });
  await openplayer.subtitle.loadGeneratedCues({
    name: "Translated Subtitles",
    format: "vtt",
    cues: [{ start: cue.start, end: cue.end, text: response.text }],
    select: true,
  });
}
```

The host writes audio clips and generated subtitle files into plugin-scoped
managed directories, formats structured `SubtitleCue[]` input as SRT/VTT, and
loads generated subtitle text through mpv. Plugins do not receive raw filesystem
write access and should not use raw mpv `sub-add` or provider-specific host
commands.

Generated subtitle tracks are plugin-owned resources. A plugin can inspect, read
back, append to, replace, or unload only tracks backed by files in its own
managed subtitle directory:

```js
const generatedTracks = await openplayer.subtitle.listGenerated();
const currentTranscript = generatedTracks.find((track) => track.selected);

if (currentTranscript) {
  const currentContent = await openplayer.subtitle.readGenerated(currentTranscript.id);

  await openplayer.subtitle.appendGeneratedCues(currentTranscript.id, {
    cues: latestTranscriptSegments,
    select: true,
  });

  await openplayer.subtitle.replaceGeneratedCues(currentTranscript.id, {
    name: "Updated Transcript",
    format: "vtt",
    cues: (currentContent.cues ?? updatedTranscriptSegments).map((cue) => ({
      ...cue,
      text: cue.text.trim(),
    })),
    select: true,
  });
}

for (const staleTrack of generatedTracks.filter((track) => !track.selected)) {
  await openplayer.subtitle.removeGenerated(staleTrack.id);
}
```

For real-time transcription, create a VTT or SRT track once with
`loadGeneratedCues`, append new `SubtitleCue[]` chunks with
`appendGeneratedCues`, and use `readGenerated` for review, translation, cleanup,
or resumable plugin state. `readGenerated` returns the raw subtitle content and
parsed `SubtitleCue[]` for SRT/VTT. The host keeps all reads and writes scoped to
the current plugin's managed subtitle files and asks mpv to reload updated
subtitle tracks.

For larger audio clips, avoid `includeBase64` and upload the managed artifact
directly:

```js
const segment = await openplayer.media.currentSegment({ before: 20, duration: 20 });
const clip = await openplayer.audio.extractClip({
  ...segment.clip,
  sampleRate: 16000,
  channels: "mono",
});

const response = await openplayer.network.request({
  url: "https://example.com/transcribe",
  method: "POST",
  headers: { "Content-Type": "audio/wav" },
  bodyFile: {
    path: clip.path,
    contentType: clip.mimeType,
  },
  timeoutMs: 30000,
});
```

`bodyFile` is limited to host-managed artifacts for the current plugin, such as
files returned by `openplayer.audio.extractClip` or
`openplayer.capture.frame`. It is not raw filesystem upload access.

For OCR or visual understanding, use the same managed artifact pattern:

```js
const frame = await openplayer.capture.frame({ format: "webp" });
const response = await openplayer.network.request({
  url: "https://example.com/vision",
  method: "POST",
  bodyFile: {
    path: frame.path,
    contentType: frame.mimeType,
  },
  timeoutMs: 30000,
});
```

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
const settings = await openplayer.plugin.getSettings();
const storageInfo = await openplayer.storage.info();
await openplayer.storage.set("state", { enabled: true });
const state = await openplayer.storage.get("state");

const response = await openplayer.network.request({
  url: "https://example.com/whep",
  method: "POST",
  headers: { "Content-Type": "application/sdp" },
  body: offerSdp,
  timeoutMs: 8000,
});

const logo = await openplayer.network.request({
  url: "https://example.com/logo.png",
  responseType: "base64",
});
const logoDataUrl = `data:${logo.headers["content-type"] || "image/png"};base64,${logo.bodyBase64}`;

const files = await openplayer.filesystem.pickMedia({ multiple: true });
const directory = await openplayer.filesystem.pickDirectory();

await openplayer.ui.toast("Plugin ready");
await openplayer.ui.openSettings("plugins");
await openplayer.ui.openPanel("playlist");
await openplayer.ui.openView("wall");
await openplayer.ui.closeView();
```

Storage is plugin-private, redb-backed, and removed when the plugin is
uninstalled. Persistent plugins should declare `contributes.storage` so the
host can initialize missing defaults on install, preserve existing values on
upgrade, and expose schema metadata through `openplayer.storage.info` for
plugin-owned migrations:

```json
{
  "contributes": {
    "storage": {
      "version": 2,
      "defaults": {
        "runtime.launchCount": 0,
        "transcript.language": "auto",
        "transcript.queue": []
      }
    }
  }
}
```

```js
const info = await openplayer.storage.info();
if (info.schemaVersion < info.manifestVersion) {
  const queue = (await openplayer.storage.get("transcript.queue")) ?? [];
  await openplayer.storage.set("transcript.queue", queue);
  await openplayer.storage.markMigrated();
}
```

## Custom Views

Custom views are best for rich plugin-owned UI. Keep these rules:

- Use host theme tokens injected as CSS variables, such as `--op-accent`,
  `--op-panel`, `--op-text`, and `--op-danger`.
- Prefer the host-injected standard UI classes before writing custom control
  CSS: `.op-view`, `.op-surface`, `.op-stack`, `.op-row`, `.op-button`,
  `.op-button--primary`, `.op-icon-button`, `.op-input`, `.op-select`,
  `.op-textarea`, `.op-list`, `.op-list-item`, `.op-badge`, and `.op-muted`.
  These classes use tokens such as `--op-accent`, `--op-control`, `--op-text`,
  `--op-line`, and `--op-radius`, so theme plugins and user accent overrides
  automatically apply to plugin views.
- For `sidePanel` views, keep the plugin surface semi-transparent and derive
  layered backgrounds from host tokens such as `--op-panel-strong`,
  `--op-panel`, and `--op-control`, usually with `color-mix(..., transparent)`.
  The host already provides the right-side size, margins, and 14px rounded
  clipping, so the view should normally use `width: 100%`, `height: 100%`, a
  transparent document background, and no extra outer padding.
- If a `sidePanel` view needs user-tunable transparency, declare a bounded
  `number` setting in plugin settings and reference it from the view with
  `frameOpacitySetting`. OpenPlayer applies that value as host-level iframe
  opacity so WebView2 transparent subframe composition still blends with the
  native mpv video surface:

```json
{
  "settings": [
    {
      "id": "panel-opacity",
      "label": "Panel opacity",
      "kind": "number",
      "placement": "pluginSettings",
      "defaultValue": 0.82,
      "min": 0.45,
      "max": 1,
      "step": 0.05
    }
  ],
  "views": [
    {
      "id": "channels",
      "title": "Channels",
      "presentation": "sidePanel",
      "frameOpacitySetting": "panel-opacity",
      "entry": "view/index.html"
    }
  ]
}
```

- Set view `presentation` to `sidePanel` for playlist-like transparent right
  panels. The default `overlay` presentation keeps a full-stage custom view.
- HTTPS images are allowed for passive artwork such as channel logos. Direct
  HTTP requests from views remain blocked; use `openplayer.network.request` for
  playlist, API, and signaling requests.
- Use `responseType: "base64"` for small binary assets that need to be rendered
  as `data:` URLs in a custom view.
- Route all privileged actions through `openplayer`; do not use direct fetch
  for host-mediated network operations unless the feature is intentionally
  browser-local.
- When a view overlays native mpv wall tiles, call
  `openplayer.player.wall.setVisible(false)` before opening plugin-owned modal
  UI and restore visibility after closing it.
- Close native resources in `beforeunload` and when the view closes.

For action icons, use `tv` for TV-like channel browsers, IPTV surfaces, and
other television-oriented plugins. Keep `stream` for generic network stream
entry points.

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
