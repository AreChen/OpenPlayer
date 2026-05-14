# OpenPlayer Storage Recent And Progress Design Spec

Date: 2026-05-14

## Purpose

This slice adds SQLite-backed local persistence for the desktop player. It connects the new real local-path queue to durable recent media, playback progress, and a minimal settings table so later player features can build on stable storage instead of transient React state.

## Scope

Included:

- SQLite storage foundation in `crates/storage`.
- Versioned migrations for the first storage schema.
- Recent media persistence for real local file paths.
- Playback progress persistence for real local file paths.
- Minimal key/value settings persistence.
- Tauri commands for recent media, playback progress, and settings.
- UI wiring for recording recent media, showing recent media shortcuts, saving progress, and auto-resuming saved progress.

Excluded:

- Persisting browser drag/drop preview files without real paths.
- Folder history.
- HTTP URL history.
- Manual clear-history UI.
- Full settings page.
- Playlist persistence.
- Cloud sync or media library scanning.

## Storage Schema

SQLite stores three tables for this slice.

```sql
CREATE TABLE recent_media (
  path TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  last_opened_at_ms INTEGER NOT NULL,
  open_count INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE playback_progress (
  path TEXT PRIMARY KEY NOT NULL,
  position_ms INTEGER NOT NULL,
  duration_ms INTEGER,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
```

The first migration is idempotent and is run when the desktop app opens the database. Future schema changes must use additional migrations instead of changing the first migration in place after it ships.

## Rust Storage Architecture

`crates/storage` becomes the storage boundary. It owns SQLite connection setup, migrations, repository types, DTO-like domain structs, and storage errors.

Primary units:

- `StorageConfig`: chooses database location. Tests can use in-memory databases.
- `StorageDatabase`: owns a SQLite connection handle and runs migrations on open.
- `RecentMediaRepository`: records and lists recent local media.
- `PlaybackProgressRepository`: saves, loads, and clears progress by path.
- `SettingsRepository`: gets and sets string settings by key.
- `StorageError`: maps SQLite and validation failures into typed errors.

The repositories should not depend on Tauri, React DTOs, or media backend crates. They accept stable local path strings and return plain Rust structs.

## Tauri Boundary

The desktop app adds a managed storage state. It opens the SQLite database at startup and exposes stable commands:

- `storage_recent_media_list(limit?: number)` returns recent local files ordered by `last_opened_at_ms DESC`.
- `storage_recent_media_record(path, name)` upserts a recent media row and increments `open_count` on repeat opens.
- `storage_progress_get(path)` returns saved playback progress or `null`.
- `storage_progress_save(path, positionMs, durationMs?)` saves progress.
- `storage_progress_clear(path)` removes saved progress.
- `storage_setting_get(key)` returns a string value or `null`.
- `storage_setting_set(key, value)` saves a string setting.

Command errors map to stable user-safe codes, for example `storage.unavailable`, `storage.invalidInput`, and `storage.queryFailed`. Raw SQLite errors should not be exposed directly to the frontend.

## UI Data Flow

Recent media:

1. On app startup, load recent media through `storage_recent_media_list`.
2. When opening a native picker item with `sourceKind: "localFilePath"`, call `storage_recent_media_record`.
3. Refresh the recent list after recording.
4. Show recent media shortcuts in the empty state and playlist drawer.
5. Clicking a recent item creates a single-item queue using the real path and loads it.

Playback progress:

1. Track progress only for media items with a real local path.
2. Save progress after meaningful playback movement, not on every `timeupdate` event. A small debounce or interval is sufficient.
3. Save `position_ms` and `duration_ms` when duration is known.
4. When a real local path media item loads metadata, read saved progress.
5. If the saved position is valid and not near the start or end, automatically seek to it.
6. When playback reaches the end, clear saved progress for that path.

Settings:

This slice only proves the settings persistence boundary. It does not require a settings page. The UI may use it for one small setting if useful, but repository and command tests are the primary acceptance criteria.

## Resume Rules

Auto-resume applies only when all of the following are true:

- Current media item has `sourceKind: "localFilePath"`.
- Saved `position_ms` is greater than 10 seconds.
- If duration is known, saved position is at least 10 seconds before the end.
- Saved position is less than the current loaded media duration when duration is available.

If any rule fails, playback starts from the beginning and no error is shown.

## Error Handling

- Storage initialization failure should not crash the app. The app should continue playback with persistence disabled and show a user-safe error if a storage-backed action is attempted.
- Individual storage command failures should use existing `playback-error` display behavior or console logging depending on visibility needs.
- Invalid empty paths or keys should return `storage.invalidInput`.
- SQLite query/migration failures should return `storage.queryFailed` or `storage.unavailable` without raw database details in UI messages.

## Testing Strategy

Rust tests:

- Migrations create all required tables.
- Recent media upsert increments `open_count` and updates `last_opened_at_ms`.
- Recent media list respects ordering and limit.
- Playback progress saves, loads, updates, and clears by path.
- Settings get/set roundtrip works.
- Invalid inputs map to typed storage errors.
- Tauri storage commands map repository results to stable DTOs and stable errors.

Frontend verification:

- Shell verification checks storage command names, recent media state, recent item click wiring, progress save/load wiring, and auto-resume helper presence.
- TypeScript build verifies DTO names and command payload shape.

Full verification:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run verify:shell`
- `npm run build`

## Acceptance Criteria

- Desktop app initializes SQLite storage without blocking playback if storage fails.
- Opening a native local path records it in recent media.
- Recent media shortcuts are visible when available and can load a file by path.
- Playback progress for real local paths is saved during playback.
- Reopening the same path auto-resumes when saved progress passes the resume rules.
- Playback ending clears progress for that path.
- Browser drag/drop preview still works but is not persisted.
- Existing native picker queue, auto-advance, window controls, seek, volume, fullscreen, and playback command mirroring do not regress.

## Follow-Up Work

- Persist full playlists and queue state.
- Add a settings page backed by the settings repository.
- Add clear recent/history UI.
- Extend persistence to HTTP URLs after URL open UX lands.
- Use stored real paths directly with `libmpv` when the native backend replaces HTML5 preview playback.
