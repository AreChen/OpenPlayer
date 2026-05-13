# OpenPlayer Core Services Design

Date: 2026-05-13

## Purpose

This slice starts Phase 2 by defining the backend-neutral playback contract and a testable core playback service. It does not integrate `libmpv`, SQLite, or the React UI yet. The goal is to create the stable Rust boundary that later media backends, persistence, and Tauri commands will use.

## Scope

Included:

- Backend-neutral media source and playback types in `crates/media`.
- A minimal `MediaBackend` command surface for open, play, pause, stop, seek, and volume.
- Playback state, status, position, duration, and event models.
- A core `PlaybackService` that owns current playback state and maps service commands to a backend.
- Mock-backed tests for state transitions and error mapping.

Not included:

- Real `libmpv` integration.
- SQLite persistence.
- Tauri IPC commands for playback.
- Replacing the current HTML5 preview renderer.
- Subtitle, track, chapter, and hardware-decode details beyond reserving model space for later slices.

## Architecture

Runtime boundary for this slice:

```text
core PlaybackService -> media MediaBackend trait -> mock backend in tests
```

The UI remains on the current local preview path for now. Future UI work should call Tauri commands, and those commands should use `PlaybackService`; the UI must not call media backends directly.

## Media Contract

`crates/media` will define these core types:

- `MediaSource`: local file path, local folder path, or HTTP(S) URL.
- `PlaybackStatus`: idle, loading, ready, playing, paused, stopped, ended, error.
- `PlaybackSnapshot`: source, status, position, duration, volume, mute flag, speed, and latest error.
- `PlaybackEvent`: state changed, position changed, media opened, media ended, and backend error.
- `MediaTime`: duration wrapper in milliseconds to avoid float drift in state tests.
- `MediaError`: backend unavailable, invalid source, open failed, command failed, unsupported source.

`MediaBackend` will remain synchronous for this slice. That keeps the first service small and deterministic. A later libmpv slice can introduce async/event-thread internals behind the same trait or revise the trait once native integration proves its needs.

The initial backend methods:

```text
backend_id()
display_name()
open(source)
play()
pause()
stop()
seek(position)
set_volume(volume)
snapshot()
```

## Core Service

`crates/core` will add `PlaybackService<B: MediaBackend>`. The service will:

- Validate user-facing inputs before calling the backend.
- Call backend methods in command order.
- Keep a last-known `PlaybackSnapshot` for Tauri commands to expose later.
- Map `MediaError` into `CoreError` so higher layers do not depend on backend internals.

The service is intentionally small. It should not know about SQLite, UI layout, native windows, or plugin/theme systems.

## Error Handling

Errors should be typed and stable enough for future IPC mapping:

- Invalid local path or URL source.
- Backend command failure.
- Backend unavailable.
- Invalid seek target.
- Invalid volume value.

Tests should assert error categories, not English wording.

## Testing

Required tests:

- `MediaSource` accepts local files/folders and HTTP(S) URLs while rejecting unsupported schemes.
- Backend info is still derived from any `MediaBackend` implementation.
- Mock backend records command order for open, play, pause, stop, seek, and volume.
- `PlaybackService` transitions from idle to ready after open.
- `PlaybackService` transitions to playing and paused after commands.
- Seek and volume update the snapshot.
- Backend errors are mapped into core errors.

The tests should run with `cargo test --workspace` and must not require native media dependencies.

## Acceptance Criteria

- `crates/media` exposes backend-neutral playback types and a command-capable `MediaBackend` trait.
- `crates/core` exposes a `PlaybackService` with tests around command flow and state snapshots.
- Existing `crates/mpv::MpvBackendDescriptor` still compiles as an identity-only backend descriptor until real mpv integration starts.
- No UI or SQLite behavior regresses.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass.

## Next Slice

After this slice, the next implementation plan should connect Tauri playback commands to `PlaybackService` while the UI still uses the current preview renderer. Real `libmpv` integration should follow only after the command/state boundary is tested.
