# OpenPlayer Tauri Playback Commands Design

Date: 2026-05-14

## Purpose

This slice connects the desktop app to the Rust playback service through Tauri commands. The current HTML5 preview renderer remains responsible for actual visible playback. The Rust side will track the same high-level playback state through `PlaybackService`, proving the React -> Tauri -> core command path before real `libmpv` integration.

## Scope

Included:

- A desktop-local preview backend implementing `openplayer_media::MediaBackend`.
- Tauri-managed playback state shared by playback commands.
- Serializable DTOs for playback source, snapshot, status, and errors.
- Commands for snapshot, open local preview source, play, pause, stop, seek, and set volume.
- Frontend command calls that mirror the existing HTML5 preview actions without replacing the renderer.
- Tests for command/state behavior and source validation.

Not included:

- Real `libmpv` playback.
- Filesystem path access for browser `File` objects.
- SQLite persistence.
- Playlist persistence or recent media.
- Replacing the current `<video>` rendering path.
- Native file picker integration.

## Architecture

Runtime flow for this slice:

```text
React UI action -> Tauri command -> DesktopPlaybackState -> PlaybackService<PreviewPlaybackBackend> -> PlaybackSnapshot DTO -> React state/error handling
```

The preview backend is intentionally desktop-local. It is not a media backend for real playback; it only mirrors state transitions so the IPC boundary can be exercised safely. Real `libmpv` work will replace this backend later without changing the command names or DTO shape unnecessarily.

## Desktop Backend And State

`apps/desktop/src-tauri/src/lib.rs` will add a small `PreviewPlaybackBackend`:

- Holds a `PlaybackSnapshot`.
- `open(source)` stores the source and marks status `Ready`.
- `play()` marks `Playing` only when a source exists; otherwise returns `InvalidSource`.
- `pause()` marks `Paused`.
- `stop()` marks `Stopped` and resets position.
- `seek(position)` updates position.
- `set_volume(volume)` updates volume.
- `snapshot()` returns the current snapshot.

Tauri state will wrap `PlaybackService<PreviewPlaybackBackend>` in `Mutex` because commands mutate shared state. The state type should remain private to the desktop crate.

## Command API

Commands:

- `playback_snapshot() -> Result<PlaybackSnapshotDto, PlaybackCommandError>`
- `playback_open_preview_source(source: PlaybackSourceDto) -> Result<PlaybackSnapshotDto, PlaybackCommandError>`
- `playback_play() -> Result<PlaybackSnapshotDto, PlaybackCommandError>`
- `playback_pause() -> Result<PlaybackSnapshotDto, PlaybackCommandError>`
- `playback_stop() -> Result<PlaybackSnapshotDto, PlaybackCommandError>`
- `playback_seek(position_ms: u64) -> Result<PlaybackSnapshotDto, PlaybackCommandError>`
- `playback_set_volume(percent: u16) -> Result<PlaybackSnapshotDto, PlaybackCommandError>`

DTOs should use `serde` and camelCase fields:

- `PlaybackSourceDto`: local file label, local folder label, or HTTP URL string.
- `PlaybackSnapshotDto`: source label, status, positionMs, durationMs, volumePercent, muted, speedMilli, latestError.
- `PlaybackCommandError`: code and message.

For browser-picked local `File` objects, the UI only has a file name and object URL, not a stable native path. Therefore this slice sends a local file label to Rust, not a real filesystem path. The Rust state must treat that label as preview-only metadata.

## Frontend Integration

The UI should remain visually unchanged. Existing actions will mirror state into Rust:

- Opening or dropping a file calls `playback_open_preview_source` with the file name/type metadata.
- Play calls `playback_play` after the HTML5 player successfully starts or as part of the same user action.
- Pause calls `playback_pause`.
- Restart calls `playback_seek(0)`.
- Seek calls `playback_seek(positionMs)`.
- Volume calls `playback_set_volume(percent)`.

If a Rust command fails, the UI should show the same lightweight playback error surface already used for HTML5 decode errors. The UI should not show raw Rust debug strings.

## Error Handling

Command errors should preserve stable codes for future UI copy:

- `media.invalidSource`
- `media.unsupportedSource`
- `media.backendUnavailable`
- `media.commandFailed`
- `media.invalidSeekTarget`
- `media.invalidVolume`
- `state.lockFailed`

The DTO message can be concise and user-safe. Tests should assert codes where practical.

## Testing

Rust tests:

- `playback_snapshot` starts idle.
- Opening a preview local file label returns status `Ready` with a source label.
- Play/pause/stop commands transition state.
- Seek and volume commands update snapshot values.
- Invalid volume returns `media.invalidVolume`.
- Playing before opening a source returns `media.invalidSource`.

Frontend verification:

- `npm run verify:shell` should assert the new command names are wired in `App.tsx`.
- `npm run build` should type-check DTO usage.

Full verification:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run verify:shell`
- `npm run build`

## Acceptance Criteria

- Desktop app registers playback commands in Tauri.
- Commands use `PlaybackService` rather than duplicating playback state logic outside core.
- UI still plays through the current HTML5 preview but also exercises the Tauri command path.
- Invalid command input returns stable error codes.
- Existing shell interactions, no-console release behavior, and current preview playback are not regressed.

## Next Slice

After this slice, the next logical step is either SQLite-backed recent/progress state or native file picker/path handling. Real `libmpv` integration should wait until command/state boundaries and native dependency strategy are both stable.
