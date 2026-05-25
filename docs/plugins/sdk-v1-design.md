# OpenPlayer Plugin SDK v1 Design

## Goal

Plugin SDK v1 turns OpenPlayer plugins into first-class extensions instead of
thin switches for core player features. The host exposes stable, permissioned
capabilities; official and third-party plugins compose those capabilities from
their own packages.

## Boundaries

- The host owns security, validation, redb persistence, mpv access, native
  dialogs, filesystem access, and UI rendering.
- Plugins run in the `webviewJs` worker sandbox. They do not access the DOM,
  Tauri APIs, native files, or network directly.
- Plugins call `openplayer.request`, subscribe with `openplayer.onEvent`, modify
  media opening through `openplayer.onBeforeOpenMedia`, and register plugin-owned
  UI commands through `openplayer.registerCommand`.
- Plugin-owned commands use the `plugin.*` prefix and are routed only to the
  same plugin runtime that declared the action.

## SDK Surface

SDK v1 provides a typed wrapper over the host bridge:

- Lifecycle: `onReady`, `onEvent`, `onBeforeOpenMedia`, `registerCommand`.
- Requests: `request`, `getSettings`, `getCurrentMedia`, `getSnapshot`.
- Player: `play`, `pause`, `togglePlayback`, `stop`, `seek`, `setVolume`,
  `setSpeed`, `setLoopMode`, `setFullscreen`, `setAlwaysOnTop`.
- Media: `openMedia`, `openStream`, `openStreamDialog`.
- Capture: `captureScreenshot`, `startRecording`, `stopRecording`,
  `toggleRecording`, `recordingState`.
- Native wall: `player.wall.open`, `player.wall.layout`,
  `player.wall.snapshot`, `player.wall.setVisible`, `player.wall.close`.
- Storage: `storage.get`, `storage.set`, `storage.remove`, `storage.list`,
  isolated by plugin ID and persisted in redb.
- Network: `network.request` for bounded HTTP(S) requests, including WHEP
  signaling from custom WebRTC views.

## Permissions

- Low-risk reads and plugin-private storage require no extra permission.
- `media.openStream` gates stream opening and media path rewrites.
- `mpv.loadOptions` gates mpv `loadfile` option injection.
- `mpv.capture` gates screenshot and recording commands.
- `mpv.wall` gates native multi-stream mpv child-window walls, including tile
  open/layout/snapshot/visibility/close commands.
- `network.request` gates host-mediated HTTP requests with protocol, timeout,
  and response-size limits.

## First Implementation Slice

This iteration ships the minimum useful platform layer:

1. Host-side plugin KV storage in redb with tests.
2. Runtime bridge requests for plugin storage and expanded player controls.
3. Manifest metadata for API versioning, minimum host version, author, and
   update links.
4. Host-mediated `network.request`, UI helpers, playlist helpers, subtitle
   helpers, and user-mediated filesystem helpers.
5. TypeScript SDK package in `openplayer-plugins/packages/sdk`.
6. Runtime plugin template in `openplayer-plugins/templates/runtime-plugin`.
7. Network Stream plugin migrated to the SDK-shaped high-level API.

## Verification

- Main repo: `cargo fmt`, `cargo test -p openplayer-desktop`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `npm run build`, `npm run verify:shell`.
- Plugin repo: `npm run build`.
