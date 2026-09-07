# Architecture

OpenPlayer is a Tauri v2 desktop media player built around a native libmpv
playback host and a React control overlay.

The active app is `apps/desktop`. The Rust workspace currently contains the
desktop Tauri crate at `apps/desktop/src-tauri`.

## Runtime Split

The default playback path is the `mpv-embed` feature:

```text
main Tauri window     -> native libmpv video host
transparent overlay   -> React controls, menus, settings, shortcuts
Tauri commands        -> playback, persistence, shell integration
redb stores           -> history, resume state, preferences, themes
```

The overlay should not reintroduce browser `<video>` playback, object URLs, or
the removed mpv render API spike. Window movement, fullscreen, always-on-top,
resize, and close commands should target the main video window and keep the
overlay synchronized.

## Main boundaries

Paths below are relative to `apps/desktop`.

| Module | Responsibility |
| --- | --- |
| `src/App.tsx`, `src/components/player/PlayerOverlayApp.tsx` | Select the surface and compose focused hooks and views |
| `src/hooks/`, `src/app/` | Playback orchestration, settings sync, plugin bridges, shortcuts and pure helpers |
| `src-tauri/src/bootstrap/` | Tauri setup, managed state and feature-specific command registration |
| `src-tauri/src/window/` | Native window movement, resize, fullscreen, close and overlay synchronization |
| `src-tauri/src/mpv_embed/` | libmpv playback, snapshots, capture, audio/video exports and native stream walls |
| `src-tauri/src/playback_store/` | redb history, resume positions, streams and playback settings |
| `src-tauri/src/appearance_store/` | redb themes, preferences, plugin packages, settings, runtime storage and migration metadata |
| `src-tauri/src/plugin_network/`, `plugin_artifacts/` | Host-mediated HTTP requests and plugin-owned media artifacts |

## Playback and state updates

Playback-changing commands return backend snapshots. `useMpvSnapshotPolling`
allows one outstanding poll; commands, media changes and unmounts invalidate
stale responses without discarding a valid response merely because it is slow.

`src/app/playbackClock.ts` interpolates an authoritative playback anchor.
Only the timeline and audio time label subscribe to animation updates through
`useSyncExternalStore`. Shortcuts and plugin operations read the current clock
position when invoked. Hidden timelines unsubscribe, and hidden documents stop
requesting animation frames. The clock does not own persisted playback state.

`usePersistentStateSync` reads `appearance_sync_state` and
`playback_sync_state` initially and every 1.6 seconds. Each command opens its
database once, performs the related reads off the UI thread and releases the
handle. Overlapping sync cycles are suppressed and unchanged catalog/history
results are not reapplied. Playback settings still reconcile optimistic local
edits, retaining React state identity when unchanged. Existing single-purpose
commands remain available for other callers.

Store initialization checks table existence and types with a read transaction;
it creates tables only when missing. Database handles remain command-scoped for
multi-process access. Plugin storage prefix scans seek to the namespace and
stop at its boundary instead of scanning every plugin's keys.

## Plugin composition

SDK 1.6 provides worker runtimes, custom iframe views, theme-aware UI classes,
events, tasks, media segments, exports, subtitles, artifacts, network requests
and private redb storage. Plugins compose these primitives; provider-specific
features do not require a separate AI abstraction in the host.

Network responses are capped while reading each chunk, including responses
without `Content-Length`. The host reuses its HTTP client while keeping request
timeouts and payload validation. Invalid task updates leave the existing task
unchanged, so plugins can recover through normal error handling.

## Supporting Notes

- [Native dependencies](../native-deps/README.md) documents bundled runtime
  dependency metadata.
- [Plugin architecture](../plugins/README.md) describes host/plugin boundaries.
- [SDK 1.6 developer guide](../plugins/sdk-1.6-developer-guide.md) is the current
  public API reference.
- [Regression checks](./regression-checks.md) covers behavior tests, rendering
  measurements and native window smoke tests.
- [libmpv smoke test](./libmpv2-smoke.md) documents the optional headless
  libmpv initialization check.
