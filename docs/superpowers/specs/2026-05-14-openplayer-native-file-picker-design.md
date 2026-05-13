# OpenPlayer Native File Picker Design Spec

Date: 2026-05-14

## Purpose

This slice adds a native multi-file picker and a real local-path queue while keeping the current HTML5 preview renderer. The goal is to move local file opening away from browser-only `File` labels and toward stable local paths that later SQLite persistence and `libmpv` playback can reuse.

## Scope

Included:

- Native file picker launched from the existing open-media control.
- Multiple local file selection.
- Queue state in the React app.
- Opening the first selected file immediately.
- Auto-advance to the next queued file when playback ends.
- Playlist drawer listing every selected file and allowing manual selection.
- A new playback source kind for real local file paths.
- Existing Tauri playback command mirroring for open, play, pause, seek, volume, and stop.

Excluded:

- Folder picking.
- Persistent playlists.
- Recent media or resume-progress storage.
- Drag/drop conversion to real file paths.
- Real `libmpv` rendering.
- Native subtitle, track, or chapter handling.

## Architecture

The frontend owns picker interaction and queue state for this slice. It uses Tauri's dialog API to obtain real filesystem paths, then uses Tauri's asset URL conversion to preview the selected local path in the existing `<video>` element.

```text
Open button -> Tauri file dialog -> React queue -> convertFileSrc(path) -> HTML5 video preview
                                      |
                                      -> playback_open_preview_source(localFilePath)
```

The Rust playback boundary remains the state mirror. `DesktopPlaybackState` and `PlaybackService` receive the selected path through the existing command shape with an expanded source kind. The visible media renderer remains HTML5 preview until the later `libmpv` slice.

## Data Model

`MediaItem` becomes queue-oriented:

```ts
type MediaItem = {
  id: string;
  name: string;
  path: string | null;
  url: string;
  sourceKind: "localFilePath" | "localFileLabel";
};
```

Native picker items use:

- `path`: full local filesystem path returned by Tauri.
- `url`: `convertFileSrc(path)` for the WebView preview.
- `sourceKind`: `localFilePath`.

Browser drag/drop items remain preview-only:

- `path`: `null`.
- `url`: object URL from `URL.createObjectURL(file)`.
- `sourceKind`: `localFileLabel`.

The app stores:

- `queue: MediaItem[]`.
- `currentIndex: number | null`.

The current item is derived from those two fields.

## Playback Source DTO

`PlaybackSourceDto.kind` adds `localFilePath`. The desktop command mapping treats it as `MediaSource::local_file(path)`. The existing `localFileLabel` remains only for preview-only browser files that do not expose a stable native path.

```ts
type PlaybackSourceDto = {
  kind: "localFilePath" | "localFileLabel" | "localFolderLabel" | "httpUrl";
  value: string;
};
```

Rust mirrors the same enum variant with Serde `camelCase` naming.

## User Flow

Opening media:

1. User presses the existing open-media icon.
2. Native dialog opens with multiple selection enabled.
3. If the user cancels, app state does not change.
4. If paths are selected, the app builds a new queue.
5. The first selected item becomes current.
6. The app mirrors the current item to Rust using `playback_open_preview_source`.
7. The existing `<video>` element previews the item via `convertFileSrc(path)`.

Playlist interaction:

1. The playlist drawer shows every queued item.
2. Clicking an item changes `currentIndex`.
3. The app resets timeline state, mirrors open state to Rust, and loads that item into `<video>`.

Auto-advance:

1. When `<video>` fires `ended`, the app checks for a next queue item.
2. If there is a next item, the app advances to it and mirrors open state to Rust.
3. If the previous item was playing, the app attempts to start the next preview item and mirrors `playback_play` after preview play succeeds.
4. If there is no next item, the app mirrors `playback_stop` and leaves behavior equivalent to the current stop-at-end state.

## Error Handling

- Picker cancellation is not an error.
- Empty or unsupported selections show `No supported media file was found in that selection.`
- Dialog failures show a user-safe playback error message.
- Preview decode failures keep the queue intact and show the existing WebView decode error.
- Rust command failures continue to surface through `PlaybackCommandError` and `role="alert"`.
- Stale async playback command responses are still ignored through the existing command-id guard.

## Permissions And Dependencies

The desktop frontend needs Tauri dialog support. The implementation should add `@tauri-apps/plugin-dialog` to the desktop package, add `tauri-plugin-dialog` to the desktop Rust crate, and register the plugin in the Tauri builder. Capabilities must allow file-open dialog access with `dialog:allow-open`.

The WebView preview needs Tauri asset URL support for selected local paths. The implementation should enable Tauri's asset protocol in `tauri.conf.json` with the narrowest practical scope for local media preview on the target platform. This scope is only for preview loading; it does not mean Rust reads the media file in this slice.

No native file data is decoded by Rust in this slice. The path is used as source identity and passed through the WebView asset conversion for preview.

## Testing Strategy

Rust tests:

- `PlaybackSourceDto` converts `localFilePath` into a local file media source.
- Opening a `localFilePath` source stores the path in the returned playback snapshot.

Shell verification:

- Frontend imports and uses Tauri dialog open API.
- Frontend imports and uses `convertFileSrc`.
- Tauri config enables asset protocol support for local preview URLs.
- Frontend defines `localFilePath` in the playback source DTO.
- Frontend keeps queue and current-index state.
- Frontend wires playlist item selection.
- Frontend wires auto-advance on media end.
- Tauri builder registers the dialog plugin.

Build and regression verification:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run verify:shell`
- `npm run build`

## Acceptance Criteria

- Clicking the open-media control opens a native file picker.
- Selecting multiple supported media files replaces the queue with those files.
- The first selected file loads immediately in the existing player surface.
- The playlist drawer lists every selected file.
- Clicking a playlist item loads that item.
- When a file ends, playback advances to the next queued file when present.
- Rust playback snapshot source labels use the real selected path for native picker files.
- Existing drag/drop preview still works.
- Existing window controls, drag surface, fullscreen toggle, seek, volume, and play/pause behavior do not regress.

## Follow-Up Work

- Convert drag/drop to real paths through Tauri file-drop events.
- Store recent media and resume progress in SQLite using the real path identity.
- Replace HTML5 preview playback with `libmpv` while preserving queue and command DTO boundaries.
