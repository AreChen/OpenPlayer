# OpenPlayer Plugin Documentation Guidance

Use this directory as the source of truth when writing or reviewing OpenPlayer
plugins, SDK examples, and AI-facing plugin instructions.

## Current SDK

- Treat OpenPlayer 1.6.0 as the current SDK package generation.
- Use `docs/plugins/sdk-1.6-developer-guide.md` for the public SDK contract.
- Keep examples aligned with `openplayer-plugins/packages/sdk/index.d.ts`.
- Do not document APIs unless the host bridge, backend validation, official
  plugin examples, and tests already support them.

## Subagent Workflow

- Use subagents when plugin SDK work naturally splits into independent lanes,
  such as host bridge implementation, official SDK type updates, documentation,
  UI component review, storage lifecycle review, or verification.
- Keep write ownership disjoint when delegating. For example, one agent may
  audit `apps/desktop/src/app/pluginRuntime/` while another audits
  `docs/plugins/`; do not ask multiple agents to edit the same files in
  parallel.
- Use read-only explorer agents for gap audits, verifier agents for acceptance
  evidence, and implementation workers only when the file ownership is clear.
  Integrate and review their output locally before committing.

## Custom Views

- Prefer `presentation: "sidePanel"` for playlist-like right panels.
- Side panels should use a transparent document background and a
  semi-transparent themed app surface. When transparency should be user
  adjustable, declare a `pluginSettings` number setting and reference it with
  `frameOpacitySetting` instead of hard-coding host iframe opacity.
- Derive colors from host theme tokens such as `--op-accent`, `--op-panel`,
  `--op-panel-strong`, `--op-control`, `--op-text`, and `--op-line`.
- Prefer host-injected UI classes for reusable controls before writing custom
  CSS: `.op-view`, `.op-surface`, `.op-button`, `.op-button--primary`,
  `.op-button--ghost`, `.op-button--danger`, `.op-icon-button`, `.op-input`,
  `.op-select`, `.op-slider`, `.op-switch`, `.op-switch__thumb`,
  `.op-list-item`, `.op-badge`, `.op-table`, `.op-kbd`, and `.op-muted`. Use
  `.op-toolbar`, `.op-spacer`, `.op-field`, `.op-label`, `.op-help`,
  `.op-divider`, `.op-tabs`, `.op-tab`, `.op-progress`, and `.op-empty` for
  reusable layout and state surfaces. These classes are theme-aware and follow
  the same token layer as theme plugins.
- The host owns side panel margins, right alignment, height, and 14px rounded
  clipping. Plugin views should usually set their root app to `width: 100%`,
  `height: 100%`, and avoid extra outer padding.
- Use `color-mix(..., transparent)` when a plugin needs layered panel surfaces
  so video remains subtly visible through the plugin UI.
- Custom views should use `window.openplayer.onEvent()` and
  `window.openplayer.events.subscribe(event)` for playback-reactive UI instead
  of polling. Declare the consumed events in `runtime.events`; view
  subscriptions to undeclared events are rejected by the host. Use
  `playlist.changed` for queue browsers and `recording.changed` for capture
  panels that need reactive state.

## Manifest Style

- Use `tv` for TV-like channel browsers and IPTV controls.
- Use `stream` for generic network stream entry points.
- Keep permissions minimal and feature-detect with
  `openplayer.capabilities.has(...)` or
  `openplayer.capabilities.hasPermission(...)`.
- Do not invent `ai.*`, `transcription.*`, `translation.*`, provider-specific,
  or model-specific host permissions. Provider-backed media plugins should be
  built from generic SDK primitives; if the composition is impossible, add the
  missing primitive instead of a feature-specific wrapper.
- Treat manifest capability kinds as broad UI/discovery categories. Use
  `audioTool` or `subtitleTool` for plugin grouping, but do not add per-feature
  provider capability kinds as permission gates.
- For transcription, translation, subtitle cleanup, or OCR subtitle tools,
  compose generic permissions instead of inventing feature-specific host code:
  use `openplayer.media.currentSegment` for host-normalized time windows,
  `openplayer.media.segmentTimeline` for whole-media batch chunks,
  `audio.extract` with `openplayer.audio.extractClip` for short current media
  WAV clips, `mpv.capture` with `openplayer.capture.frame` for current video
  frame artifacts, `subtitle.read` with `openplayer.subtitle.currentCue` for
  current displayed subtitle text, `network.request` or
  `openplayer.network.requestJson` for provider calls, and `subtitle.write`
  with `openplayer.subtitle.documents.create` for timestamped `SubtitleCue[]`
  or standard subtitle text.
- Use `media.export` with `openplayer.media.exportSegment` when a plugin needs
  to save bounded audio or video clips such as MP3, WAV, MP4, or MKV into the
  user's export folder. Do not give plugins arbitrary filesystem write access
  for this workflow.
- Use `openplayer.tasks` for long-running transcription, translation, analysis,
  and batch operations. Report progress with `tasks.update`, request cooperative
  cancellation with `tasks.cancel`, and finish cancellation with
  `tasks.markCancelled`.
- Use `openplayer.log.info`, `openplayer.log.warn`, and
  `openplayer.log.error` for host-visible diagnostics in the plugin runtime log
  panel instead of relying on worker console output.
- Use `openplayer.subtitle.documents.create`, `list`, `read`, `replace`,
  `appendCues`, and `remove` when a plugin needs to create, review, update, or
  clean up its own generated subtitle documents. The older `loadGenerated*` and
  `*Generated*` helpers are compatibility aliases; new plugins should use the
  document model instead of raw mpv subtitle commands.
- Use `openplayer.subtitle.setStyle` for runtime subtitle presentation changes
  when the plugin declares `mpv.subtitleStyle`; do not route subtitle typography
  through raw `openplayer.mpv.setProperty`.
- Use `network.request` `bodyFile` for larger host-managed artifacts returned by
  APIs such as `audio.extractClip` or `capture.frame`; do not describe it as
  arbitrary local file upload access.
- Use `openplayer.artifacts.list`, `info`, `remove`, and `clear` to manage
  plugin-owned audio clips and frame captures after provider uploads or failed
  provider jobs. Audio artifact management requires `audio.extract`; frame artifact
  management requires `mpv.capture`.
- Use `openplayer.network.requestJson` for JSON provider APIs instead of
  repeating manual `JSON.stringify` and `JSON.parse`; it still requires
  `network.request`.
- Use `contributes.storage` for persistent plugin-private data. Keep defaults
  small and JSON-serializable, bump the storage `version` when changing the
  plugin's schema, read `openplayer.storage.info()` at runtime for migrations,
  use `openplayer.storage.update({ set, remove })` for atomic redb-backed
  migration/cache/queue updates, call `openplayer.storage.markMigrated()` after
  successful migration, use `openplayer.storage.list({ prefix, limit })` plus
  `storage.info().totalBytes` / `maxValueBytes` for bounded cache maintenance,
  and rely on uninstall cleanup instead of writing plugin data outside the host
  storage API.

## Verification

For SDK/docs changes, run from `apps/desktop`:

```powershell
npm run verify:shell
npm run build
```

For official plugin examples, run from `openplayer-plugins`:

```powershell
npm test
npm run build
```
