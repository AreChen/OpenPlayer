# OpenPlayer Storage Recent And Progress Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SQLite-backed recent media, playback progress, and minimal settings persistence, then wire recent shortcuts and automatic resume into the current native local-path playback flow.

**Architecture:** `crates/storage` owns SQLite setup, migrations, typed repositories, and storage errors. The desktop Tauri crate owns storage command DTOs and a managed storage state that fails safely if SQLite cannot initialize. React records recent real local paths, displays recent shortcuts, saves progress for `localFilePath` media, and auto-resumes saved progress after metadata loads.

**Tech Stack:** Rust 2024, `rusqlite` with bundled SQLite, Tauri v2 managed state and commands, React 19, TypeScript, existing native file queue and playback command mirror.

---

## Scope Check

This plan intentionally combines SQLite foundation, recent media UI, and automatic resume because they form one vertical persistence slice around real local file paths. It does not persist browser drag/drop preview files, folders, HTTP URLs, full playlists, or a settings page.

## File Structure

- Modify: `Cargo.toml` adds the workspace SQLite dependency.
- Modify: `Cargo.lock` updates dependency resolution.
- Modify: `crates/storage/Cargo.toml` uses `rusqlite`.
- Replace: `crates/storage/src/lib.rs` implements migrations, database wrapper, repositories, errors, and tests.
- Create: `apps/desktop/src-tauri/src/storage.rs` implements desktop storage state, DTOs, Tauri commands, and command tests.
- Modify: `apps/desktop/src-tauri/Cargo.toml` adds `openplayer-storage` to the desktop crate.
- Modify: `apps/desktop/src-tauri/src/lib.rs` registers storage state and commands.
- Modify: `apps/desktop/src/App.tsx` wires recent media, progress save, progress clear, and auto-resume.
- Modify: `apps/desktop/src/styles.css` styles recent shortcuts inside the existing player shell and playlist drawer.
- Modify: `apps/desktop/scripts/verify-shell.mjs` verifies storage command registration and frontend wiring.

## Task 1: Implement SQLite Storage Repositories

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/storage/Cargo.toml`
- Replace: `crates/storage/src/lib.rs`
- Modify: `Cargo.lock`

- [ ] **Step 1: Add failing storage repository tests**

Replace `crates/storage/src/lib.rs` with this test-first skeleton:

```rust
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("storage is not configured")]
    NotConfigured,
}

pub fn storage_crate_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_create_required_tables() {
        let database = StorageDatabase::in_memory().expect("database");

        assert!(database.table_exists("recent_media").expect("recent_media table"));
        assert!(database.table_exists("playback_progress").expect("playback_progress table"));
        assert!(database.table_exists("settings").expect("settings table"));
    }

    #[test]
    fn recent_media_upsert_increments_count_and_orders_by_last_opened() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();

        recent.record("C:/media/a.mp4", "a.mp4", 100).expect("record a");
        recent.record("C:/media/b.mp4", "b.mp4", 200).expect("record b");
        recent.record("C:/media/a.mp4", "a-renamed.mp4", 300).expect("record a again");

        let items = recent.list(10).expect("recent list");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].path, "C:/media/a.mp4");
        assert_eq!(items[0].name, "a-renamed.mp4");
        assert_eq!(items[0].last_opened_at_ms, 300);
        assert_eq!(items[0].open_count, 2);
        assert_eq!(items[1].path, "C:/media/b.mp4");
    }

    #[test]
    fn recent_media_list_respects_limit() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();

        recent.record("C:/media/a.mp4", "a.mp4", 100).expect("record a");
        recent.record("C:/media/b.mp4", "b.mp4", 200).expect("record b");

        let items = recent.list(1).expect("recent list");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "C:/media/b.mp4");
    }

    #[test]
    fn playback_progress_roundtrip_and_clear() {
        let database = StorageDatabase::in_memory().expect("database");
        let progress = database.playback_progress();

        progress.save("C:/media/a.mp4", 42_000, Some(120_000), 500).expect("save progress");
        let saved = progress.get("C:/media/a.mp4").expect("get progress").expect("saved progress");

        assert_eq!(saved.path, "C:/media/a.mp4");
        assert_eq!(saved.position_ms, 42_000);
        assert_eq!(saved.duration_ms, Some(120_000));
        assert_eq!(saved.updated_at_ms, 500);

        progress.clear("C:/media/a.mp4").expect("clear progress");
        assert_eq!(progress.get("C:/media/a.mp4").expect("get cleared"), None);
    }

    #[test]
    fn settings_roundtrip() {
        let database = StorageDatabase::in_memory().expect("database");
        let settings = database.settings();

        settings.set("theme", "studio-dark", 700).expect("set setting");
        let value = settings.get("theme").expect("get setting").expect("setting");

        assert_eq!(value.key, "theme");
        assert_eq!(value.value, "studio-dark");
        assert_eq!(value.updated_at_ms, 700);
    }

    #[test]
    fn repositories_reject_empty_inputs() {
        let database = StorageDatabase::in_memory().expect("database");

        assert_eq!(database.recent_media().record("", "movie.mp4", 1), Err(StorageError::InvalidInput("path")));
        assert_eq!(database.recent_media().record("C:/media/a.mp4", "", 1), Err(StorageError::InvalidInput("name")));
        assert_eq!(database.playback_progress().save("", 1, None, 1), Err(StorageError::InvalidInput("path")));
        assert_eq!(database.settings().set("", "value", 1), Err(StorageError::InvalidInput("key")));
    }
}
```

- [ ] **Step 2: Run storage tests and verify RED**

Run:

```powershell
cargo test -p openplayer-storage
```

Working directory: repository root

Expected: FAIL to compile because `StorageDatabase`, repository types, and `StorageError::InvalidInput` do not exist.

- [ ] **Step 3: Add SQLite dependency configuration**

In the root `Cargo.toml`, add `rusqlite` to `[workspace.dependencies]`:

```toml
[workspace.dependencies]
async-trait = "0.1"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
url = "2"
```

In `crates/storage/Cargo.toml`, update dependencies to:

```toml
[dependencies]
rusqlite.workspace = true
thiserror.workspace = true
```

- [ ] **Step 4: Implement storage database and repositories**

Replace `crates/storage/src/lib.rs` with:

```rust
use std::{path::Path, sync::Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

const MIGRATIONS: &[&str] = &[r#"
CREATE TABLE IF NOT EXISTS recent_media (
  path TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  last_opened_at_ms INTEGER NOT NULL,
  open_count INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS playback_progress (
  path TEXT PRIMARY KEY NOT NULL,
  position_ms INTEGER NOT NULL,
  duration_ms INTEGER,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
"#];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("storage is not configured")]
    NotConfigured,
    #[error("invalid storage input: {0}")]
    InvalidInput(&'static str),
    #[error("storage query failed: {0}")]
    QueryFailed(String),
    #[error("storage lock failed")]
    LockFailed,
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::QueryFailed(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentMedia {
    pub path: String,
    pub name: String,
    pub last_opened_at_ms: i64,
    pub open_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackProgress {
    pub path: String,
    pub position_ms: i64,
    pub duration_ms: Option<i64>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingValue {
    pub key: String,
    pub value: String,
    pub updated_at_ms: i64,
}

#[derive(Debug)]
pub struct StorageDatabase {
    connection: Mutex<Connection>,
}

impl StorageDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path).map_err(StorageError::from)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory().map_err(StorageError::from)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self, StorageError> {
        let database = Self {
            connection: Mutex::new(connection),
        };
        database.run_migrations()?;
        Ok(database)
    }

    pub fn recent_media(&self) -> RecentMediaRepository<'_> {
        RecentMediaRepository { database: self }
    }

    pub fn playback_progress(&self) -> PlaybackProgressRepository<'_> {
        PlaybackProgressRepository { database: self }
    }

    pub fn settings(&self) -> SettingsRepository<'_> {
        SettingsRepository { database: self }
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool, StorageError> {
        self.with_connection(|connection| {
            let exists = connection
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
                    params![table_name],
                    |_| Ok(()),
                )
                .optional()
                .map_err(StorageError::from)?
                .is_some();
            Ok(exists)
        })
    }

    fn run_migrations(&self) -> Result<(), StorageError> {
        self.with_connection(|connection| {
            connection.execute_batch("PRAGMA foreign_keys = ON;").map_err(StorageError::from)?;
            for migration in MIGRATIONS {
                connection.execute_batch(migration).map_err(StorageError::from)?;
            }
            Ok(())
        })
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let connection = self.connection.lock().map_err(|_| StorageError::LockFailed)?;
        operation(&connection)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RecentMediaRepository<'a> {
    database: &'a StorageDatabase,
}

impl RecentMediaRepository<'_> {
    pub fn record(&self, path: &str, name: &str, now_ms: i64) -> Result<(), StorageError> {
        validate_non_empty(path, "path")?;
        validate_non_empty(name, "name")?;

        self.database.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO recent_media (path, name, last_opened_at_ms, open_count)
                     VALUES (?1, ?2, ?3, 1)
                     ON CONFLICT(path) DO UPDATE SET
                       name = excluded.name,
                       last_opened_at_ms = excluded.last_opened_at_ms,
                       open_count = recent_media.open_count + 1",
                    params![path, name, now_ms],
                )
                .map_err(StorageError::from)?;
            Ok(())
        })
    }

    pub fn list(&self, limit: u32) -> Result<Vec<RecentMedia>, StorageError> {
        let limit = i64::from(limit.clamp(1, 100));
        self.database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT path, name, last_opened_at_ms, open_count
                     FROM recent_media
                     ORDER BY last_opened_at_ms DESC
                     LIMIT ?1",
                )
                .map_err(StorageError::from)?;
            let rows = statement
                .query_map(params![limit], |row| {
                    Ok(RecentMedia {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        last_opened_at_ms: row.get(2)?,
                        open_count: row.get(3)?,
                    })
                })
                .map_err(StorageError::from)?;

            rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlaybackProgressRepository<'a> {
    database: &'a StorageDatabase,
}

impl PlaybackProgressRepository<'_> {
    pub fn save(
        &self,
        path: &str,
        position_ms: i64,
        duration_ms: Option<i64>,
        now_ms: i64,
    ) -> Result<(), StorageError> {
        validate_non_empty(path, "path")?;
        if position_ms < 0 {
            return Err(StorageError::InvalidInput("position_ms"));
        }
        if duration_ms.is_some_and(|duration| duration < 0) {
            return Err(StorageError::InvalidInput("duration_ms"));
        }

        self.database.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO playback_progress (path, position_ms, duration_ms, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(path) DO UPDATE SET
                       position_ms = excluded.position_ms,
                       duration_ms = excluded.duration_ms,
                       updated_at_ms = excluded.updated_at_ms",
                    params![path, position_ms, duration_ms, now_ms],
                )
                .map_err(StorageError::from)?;
            Ok(())
        })
    }

    pub fn get(&self, path: &str) -> Result<Option<PlaybackProgress>, StorageError> {
        validate_non_empty(path, "path")?;
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT path, position_ms, duration_ms, updated_at_ms
                     FROM playback_progress
                     WHERE path = ?1",
                    params![path],
                    |row| {
                        Ok(PlaybackProgress {
                            path: row.get(0)?,
                            position_ms: row.get(1)?,
                            duration_ms: row.get(2)?,
                            updated_at_ms: row.get(3)?,
                        })
                    },
                )
                .optional()
                .map_err(StorageError::from)
        })
    }

    pub fn clear(&self, path: &str) -> Result<(), StorageError> {
        validate_non_empty(path, "path")?;
        self.database.with_connection(|connection| {
            connection
                .execute("DELETE FROM playback_progress WHERE path = ?1", params![path])
                .map_err(StorageError::from)?;
            Ok(())
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SettingsRepository<'a> {
    database: &'a StorageDatabase,
}

impl SettingsRepository<'_> {
    pub fn set(&self, key: &str, value: &str, now_ms: i64) -> Result<(), StorageError> {
        validate_non_empty(key, "key")?;
        self.database.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO settings (key, value, updated_at_ms)
                     VALUES (?1, ?2, ?3)
                     ON CONFLICT(key) DO UPDATE SET
                       value = excluded.value,
                       updated_at_ms = excluded.updated_at_ms",
                    params![key, value, now_ms],
                )
                .map_err(StorageError::from)?;
            Ok(())
        })
    }

    pub fn get(&self, key: &str) -> Result<Option<SettingValue>, StorageError> {
        validate_non_empty(key, "key")?;
        self.database.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT key, value, updated_at_ms FROM settings WHERE key = ?1",
                    params![key],
                    |row| {
                        Ok(SettingValue {
                            key: row.get(0)?,
                            value: row.get(1)?,
                            updated_at_ms: row.get(2)?,
                        })
                    },
                )
                .optional()
                .map_err(StorageError::from)
        })
    }
}

pub fn storage_crate_ready() -> bool {
    true
}

fn validate_non_empty(value: &str, field: &'static str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        Err(StorageError::InvalidInput(field))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_create_required_tables() {
        let database = StorageDatabase::in_memory().expect("database");

        assert!(database.table_exists("recent_media").expect("recent_media table"));
        assert!(database.table_exists("playback_progress").expect("playback_progress table"));
        assert!(database.table_exists("settings").expect("settings table"));
    }

    #[test]
    fn recent_media_upsert_increments_count_and_orders_by_last_opened() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();

        recent.record("C:/media/a.mp4", "a.mp4", 100).expect("record a");
        recent.record("C:/media/b.mp4", "b.mp4", 200).expect("record b");
        recent.record("C:/media/a.mp4", "a-renamed.mp4", 300).expect("record a again");

        let items = recent.list(10).expect("recent list");

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].path, "C:/media/a.mp4");
        assert_eq!(items[0].name, "a-renamed.mp4");
        assert_eq!(items[0].last_opened_at_ms, 300);
        assert_eq!(items[0].open_count, 2);
        assert_eq!(items[1].path, "C:/media/b.mp4");
    }

    #[test]
    fn recent_media_list_respects_limit() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();

        recent.record("C:/media/a.mp4", "a.mp4", 100).expect("record a");
        recent.record("C:/media/b.mp4", "b.mp4", 200).expect("record b");

        let items = recent.list(1).expect("recent list");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "C:/media/b.mp4");
    }

    #[test]
    fn playback_progress_roundtrip_and_clear() {
        let database = StorageDatabase::in_memory().expect("database");
        let progress = database.playback_progress();

        progress.save("C:/media/a.mp4", 42_000, Some(120_000), 500).expect("save progress");
        let saved = progress.get("C:/media/a.mp4").expect("get progress").expect("saved progress");

        assert_eq!(saved.path, "C:/media/a.mp4");
        assert_eq!(saved.position_ms, 42_000);
        assert_eq!(saved.duration_ms, Some(120_000));
        assert_eq!(saved.updated_at_ms, 500);

        progress.clear("C:/media/a.mp4").expect("clear progress");
        assert_eq!(progress.get("C:/media/a.mp4").expect("get cleared"), None);
    }

    #[test]
    fn settings_roundtrip() {
        let database = StorageDatabase::in_memory().expect("database");
        let settings = database.settings();

        settings.set("theme", "studio-dark", 700).expect("set setting");
        let value = settings.get("theme").expect("get setting").expect("setting");

        assert_eq!(value.key, "theme");
        assert_eq!(value.value, "studio-dark");
        assert_eq!(value.updated_at_ms, 700);
    }

    #[test]
    fn repositories_reject_empty_inputs() {
        let database = StorageDatabase::in_memory().expect("database");

        assert_eq!(database.recent_media().record("", "movie.mp4", 1), Err(StorageError::InvalidInput("path")));
        assert_eq!(database.recent_media().record("C:/media/a.mp4", "", 1), Err(StorageError::InvalidInput("name")));
        assert_eq!(database.playback_progress().save("", 1, None, 1), Err(StorageError::InvalidInput("path")));
        assert_eq!(database.settings().set("", "value", 1), Err(StorageError::InvalidInput("key")));
    }
}
```

- [ ] **Step 5: Run storage tests and verify GREEN**

Run:

```powershell
cargo test -p openplayer-storage
```

Expected: PASS for storage repository tests.

- [ ] **Step 6: Commit storage repository changes**

Run:

```powershell
git add Cargo.toml Cargo.lock crates/storage/Cargo.toml crates/storage/src/lib.rs
git commit -m "feat: add SQLite storage repositories"
```

Expected: commit succeeds and contains only workspace dependency, storage crate, and lockfile changes.

## Task 2: Add Desktop Storage State And Commands

**Files:**
- Create: `apps/desktop/src-tauri/src/storage.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/scripts/verify-shell.mjs`
- Modify: `Cargo.lock`

- [ ] **Step 1: Add failing shell assertions for storage command registration**

In `apps/desktop/scripts/verify-shell.mjs`, add this command list after `frontendPlaybackCommands`:

```js
const storageCommands = [
  "storage_recent_media_list",
  "storage_recent_media_record",
  "storage_progress_get",
  "storage_progress_save",
  "storage_progress_clear",
  "storage_setting_get",
  "storage_setting_set",
];
```

Add these assertions after the playback command registration assertions:

```js
assert.match(tauriLibSource, /mod storage;/, "desktop app must include storage command module");
assert.match(tauriLibSource, /DesktopStorageState/, "desktop app must manage storage state");
for (const command of storageCommands) {
  assert.match(tauriGenerateHandler, new RegExp(command), `Tauri must register ${command}`);
}
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL because storage module and commands are not registered.

- [ ] **Step 3: Add desktop storage dependency**

In `apps/desktop/src-tauri/Cargo.toml`, update dependencies to include `openplayer-storage`:

```toml
[dependencies]
openplayer-core = { path = "../../../crates/core" }
openplayer-media = { path = "../../../crates/media" }
openplayer-shared = { path = "../../../crates/shared" }
openplayer-storage = { path = "../../../crates/storage" }
serde.workspace = true
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-dialog = "2"
```

- [ ] **Step 4: Create desktop storage command module**

Create `apps/desktop/src-tauri/src/storage.rs` with:

```rust
use std::{path::PathBuf, sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use openplayer_storage::{PlaybackProgress, RecentMedia, StorageDatabase, StorageError};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug)]
pub struct DesktopStorageState {
    database: Option<Arc<StorageDatabase>>,
    init_error: Option<String>,
}

impl DesktopStorageState {
    pub fn open(path: PathBuf) -> Self {
        match StorageDatabase::open(path) {
            Ok(database) => Self {
                database: Some(Arc::new(database)),
                init_error: None,
            },
            Err(error) => Self::unavailable(error.to_string()),
        }
    }

    pub fn in_memory_for_tests() -> Self {
        Self {
            database: Some(Arc::new(StorageDatabase::in_memory().expect("in-memory database"))),
            init_error: None,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            database: None,
            init_error: Some(message.into()),
        }
    }

    pub fn list_recent_media(&self, limit: Option<u32>) -> Result<Vec<RecentMediaDto>, StorageCommandError> {
        let limit = limit.unwrap_or(12);
        let database = self.database()?;
        database
            .recent_media()
            .list(limit)
            .map(|items| items.into_iter().map(RecentMediaDto::from).collect())
            .map_err(StorageCommandError::from)
    }

    pub fn record_recent_media(&self, path: String, name: String) -> Result<RecentMediaDto, StorageCommandError> {
        let database = self.database()?;
        database.recent_media().record(&path, &name, now_millis()?)?;
        let item = database
            .recent_media()
            .list(100)?
            .into_iter()
            .find(|item| item.path == path)
            .ok_or_else(|| StorageCommandError::new("storage.queryFailed", "Recent media could not be loaded"))?;
        Ok(RecentMediaDto::from(item))
    }

    pub fn get_progress(&self, path: String) -> Result<Option<PlaybackProgressDto>, StorageCommandError> {
        let database = self.database()?;
        database
            .playback_progress()
            .get(&path)
            .map(|progress| progress.map(PlaybackProgressDto::from))
            .map_err(StorageCommandError::from)
    }

    pub fn save_progress(
        &self,
        path: String,
        position_ms: i64,
        duration_ms: Option<i64>,
    ) -> Result<(), StorageCommandError> {
        let database = self.database()?;
        database
            .playback_progress()
            .save(&path, position_ms, duration_ms, now_millis()?)
            .map_err(StorageCommandError::from)
    }

    pub fn clear_progress(&self, path: String) -> Result<(), StorageCommandError> {
        let database = self.database()?;
        database.playback_progress().clear(&path).map_err(StorageCommandError::from)
    }

    pub fn get_setting(&self, key: String) -> Result<Option<String>, StorageCommandError> {
        let database = self.database()?;
        database
            .settings()
            .get(&key)
            .map(|setting| setting.map(|setting| setting.value))
            .map_err(StorageCommandError::from)
    }

    pub fn set_setting(&self, key: String, value: String) -> Result<(), StorageCommandError> {
        let database = self.database()?;
        database.settings().set(&key, &value, now_millis()?).map_err(StorageCommandError::from)
    }

    fn database(&self) -> Result<&StorageDatabase, StorageCommandError> {
        self.database
            .as_deref()
            .ok_or_else(|| StorageCommandError::new("storage.unavailable", self.init_error.as_deref().unwrap_or("Storage is unavailable")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentMediaDto {
    pub path: String,
    pub name: String,
    pub last_opened_at_ms: i64,
    pub open_count: i64,
}

impl From<RecentMedia> for RecentMediaDto {
    fn from(item: RecentMedia) -> Self {
        Self {
            path: item.path,
            name: item.name,
            last_opened_at_ms: item.last_opened_at_ms,
            open_count: item.open_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackProgressDto {
    pub path: String,
    pub position_ms: i64,
    pub duration_ms: Option<i64>,
    pub updated_at_ms: i64,
}

impl From<PlaybackProgress> for PlaybackProgressDto {
    fn from(progress: PlaybackProgress) -> Self {
        Self {
            path: progress.path,
            position_ms: progress.position_ms,
            duration_ms: progress.duration_ms,
            updated_at_ms: progress.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageCommandError {
    pub code: String,
    pub message: String,
}

impl StorageCommandError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<StorageError> for StorageCommandError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::NotConfigured => Self::new("storage.unavailable", "Storage is unavailable"),
            StorageError::InvalidInput(_) => Self::new("storage.invalidInput", "Storage input is invalid"),
            StorageError::QueryFailed(_) | StorageError::LockFailed => {
                Self::new("storage.queryFailed", "Storage query failed")
            }
        }
    }
}

#[tauri::command]
pub fn storage_recent_media_list(
    state: State<'_, DesktopStorageState>,
    limit: Option<u32>,
) -> Result<Vec<RecentMediaDto>, StorageCommandError> {
    state.list_recent_media(limit)
}

#[tauri::command]
pub fn storage_recent_media_record(
    state: State<'_, DesktopStorageState>,
    path: String,
    name: String,
) -> Result<RecentMediaDto, StorageCommandError> {
    state.record_recent_media(path, name)
}

#[tauri::command]
pub fn storage_progress_get(
    state: State<'_, DesktopStorageState>,
    path: String,
) -> Result<Option<PlaybackProgressDto>, StorageCommandError> {
    state.get_progress(path)
}

#[tauri::command]
pub fn storage_progress_save(
    state: State<'_, DesktopStorageState>,
    path: String,
    position_ms: i64,
    duration_ms: Option<i64>,
) -> Result<(), StorageCommandError> {
    state.save_progress(path, position_ms, duration_ms)
}

#[tauri::command]
pub fn storage_progress_clear(
    state: State<'_, DesktopStorageState>,
    path: String,
) -> Result<(), StorageCommandError> {
    state.clear_progress(path)
}

#[tauri::command]
pub fn storage_setting_get(
    state: State<'_, DesktopStorageState>,
    key: String,
) -> Result<Option<String>, StorageCommandError> {
    state.get_setting(key)
}

#[tauri::command]
pub fn storage_setting_set(
    state: State<'_, DesktopStorageState>,
    key: String,
    value: String,
) -> Result<(), StorageCommandError> {
    state.set_setting(key, value)
}

fn now_millis() -> Result<i64, StorageCommandError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StorageCommandError::new("storage.invalidClock", "System clock is invalid"))?;
    Ok(duration.as_millis().min(i64::MAX as u128) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_media_commands_record_and_list_items() {
        let state = DesktopStorageState::in_memory_for_tests();

        let recorded = state
            .record_recent_media("C:/media/a.mp4".to_string(), "a.mp4".to_string())
            .expect("record recent");
        let items = state.list_recent_media(Some(10)).expect("list recent");

        assert_eq!(recorded.path, "C:/media/a.mp4");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "a.mp4");
    }

    #[test]
    fn progress_commands_save_get_and_clear() {
        let state = DesktopStorageState::in_memory_for_tests();

        state
            .save_progress("C:/media/a.mp4".to_string(), 15_000, Some(60_000))
            .expect("save progress");
        let progress = state
            .get_progress("C:/media/a.mp4".to_string())
            .expect("get progress")
            .expect("progress");

        assert_eq!(progress.position_ms, 15_000);
        assert_eq!(progress.duration_ms, Some(60_000));

        state.clear_progress("C:/media/a.mp4".to_string()).expect("clear progress");
        assert_eq!(state.get_progress("C:/media/a.mp4".to_string()).expect("get cleared"), None);
    }

    #[test]
    fn setting_commands_roundtrip() {
        let state = DesktopStorageState::in_memory_for_tests();

        state
            .set_setting("theme".to_string(), "studio-dark".to_string())
            .expect("set setting");
        let loaded = state
            .get_setting("theme".to_string())
            .expect("get setting")
            .expect("setting");

        assert_eq!(loaded, "studio-dark");
    }

    #[test]
    fn unavailable_storage_maps_to_stable_error() {
        let state = DesktopStorageState::unavailable("database open failed");

        let error = state.list_recent_media(Some(10)).expect_err("unavailable storage");

        assert_eq!(error.code, "storage.unavailable");
    }

    #[test]
    fn invalid_input_maps_to_stable_error() {
        let state = DesktopStorageState::in_memory_for_tests();

        let error = state
            .record_recent_media("".to_string(), "a.mp4".to_string())
            .expect_err("invalid input");

        assert_eq!(error.code, "storage.invalidInput");
    }
}
```

- [ ] **Step 5: Wire storage state and commands into Tauri**

In `apps/desktop/src-tauri/src/lib.rs`, add `mod storage;` below `mod playback;`, import storage items, and update `run()`.

The top of the file should include:

```rust
mod playback;
mod storage;

use openplayer_shared::AppInfo;
use playback::{
    DesktopPlaybackState, playback_open_preview_source, playback_pause, playback_play,
    playback_seek, playback_set_volume, playback_snapshot, playback_stop,
};
use storage::{
    DesktopStorageState, storage_progress_clear, storage_progress_get, storage_progress_save,
    storage_recent_media_list, storage_recent_media_record, storage_setting_get,
    storage_setting_set,
};
use tauri::{Manager, Window};
```

Replace `run()` with:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let storage_state = match app.path().app_data_dir() {
                Ok(app_data_dir) => match std::fs::create_dir_all(&app_data_dir) {
                    Ok(()) => DesktopStorageState::open(app_data_dir.join("openplayer.sqlite3")),
                    Err(error) => DesktopStorageState::unavailable(error.to_string()),
                },
                Err(error) => DesktopStorageState::unavailable(error.to_string()),
            };
            app.manage(storage_state);
            Ok(())
        })
        .manage(DesktopPlaybackState::default())
        .invoke_handler(tauri::generate_handler![
            app_health_command,
            window_minimize,
            window_toggle_maximize,
            window_close,
            playback_snapshot,
            playback_open_preview_source,
            playback_play,
            playback_pause,
            playback_stop,
            playback_seek,
            playback_set_volume,
            storage_recent_media_list,
            storage_recent_media_record,
            storage_progress_get,
            storage_progress_save,
            storage_progress_clear,
            storage_setting_get,
            storage_setting_set
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
```

- [ ] **Step 6: Run desktop storage tests and shell verification**

Run:

```powershell
cargo test -p openplayer-desktop storage
npm run verify:shell
```

Working directories:

- `cargo test -p openplayer-desktop storage`: repository root
- `npm run verify:shell`: `apps/desktop`

Expected: both PASS.

- [ ] **Step 7: Commit desktop storage command changes**

Run:

```powershell
git add Cargo.lock apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/src/storage.rs apps/desktop/scripts/verify-shell.mjs
git commit -m "feat: add desktop storage commands"
```

Expected: commit succeeds and contains only desktop storage command, registration, verification, and lockfile changes.

## Task 3: Wire Recent Media And Resume Into React

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Add failing frontend shell assertions**

In `apps/desktop/scripts/verify-shell.mjs`, add these assertions near the existing frontend playback assertions:

```js
assert.match(appSource, /type RecentMediaDto/, "frontend must define recent media DTO");
assert.match(appSource, /type PlaybackProgressDto/, "frontend must define playback progress DTO");
assert.match(appSource, /runStorageCommand/, "frontend must use a storage command helper");
assert.match(appSource, /storage_recent_media_list/, "frontend must load recent media from storage");
assert.match(appSource, /storage_recent_media_record/, "frontend must record native media in recent storage");
assert.match(appSource, /storage_progress_get/, "frontend must load playback progress for auto-resume");
assert.match(appSource, /storage_progress_save/, "frontend must save playback progress");
assert.match(appSource, /storage_progress_clear/, "frontend must clear playback progress at media end");
assert.match(appSource, /recentMedia/, "frontend must keep recent media state");
assert.match(appSource, /openRecentMedia/, "frontend must open recent media shortcuts");
assert.match(appSource, /maybeResumePlayback/, "frontend must auto-resume saved progress");
assert.match(appSource, /maybeSavePlaybackProgress/, "frontend must throttle progress saves");
assert.match(styles, /recent-shortcuts/, "styles must include recent media shortcuts");
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL because recent/progress frontend wiring is not implemented.

- [ ] **Step 3: Add frontend storage DTOs and helpers**

In `apps/desktop/src/App.tsx`, add these DTOs after `PlaybackCommandError`:

```ts
type RecentMediaDto = {
  path: string;
  name: string;
  lastOpenedAtMs: number;
  openCount: number;
};

type PlaybackProgressDto = {
  path: string;
  positionMs: number;
  durationMs: number | null;
  updatedAtMs: number;
};

type StorageCommandError = {
  code: string;
  message: string;
};
```

Add this helper below `runPlaybackCommand`:

```ts
function storageErrorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as StorageCommandError).message);
  }
  return error instanceof Error ? error.message : String(error);
}

function runStorageCommand<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args).catch((error: unknown) => {
    throw new Error(storageErrorMessage(error));
  });
}
```

Change `mediaItemFromNativePath` to accept an optional display name:

```ts
function mediaItemFromNativePath(path: string, index: number, displayName = fileNameFromPath(path)): MediaItem {
  return {
    id: nextMediaItemId("native"),
    name: displayName,
    path,
    type: "media file",
    size: null,
    url: convertFileSrc(path),
    sourceKind: "localFilePath",
  };
}
```

- [ ] **Step 4: Add recent/progress state and startup load**

Inside `App()`, add these refs and state next to the existing state declarations:

```ts
const [recentMedia, setRecentMedia] = useState<RecentMediaDto[]>([]);
const currentMediaIdRef = useRef<string | null>(null);
const resumedMediaIdRef = useRef<string | null>(null);
const lastProgressSaveRef = useRef<{ mediaId: string; positionMs: number } | null>(null);
```

Add this effect after `const media = ...`:

```ts
useEffect(() => {
  currentMediaIdRef.current = media?.id ?? null;
}, [media?.id]);

useEffect(() => {
  void refreshRecentMedia();
}, []);
```

Add these functions before `mirrorPlaybackCommand`:

```ts
function refreshRecentMedia() {
  return runStorageCommand<RecentMediaDto[]>("storage_recent_media_list", { limit: 12 })
    .then(setRecentMedia)
    .catch((error: unknown) => {
      console.error("Recent media load failed", error);
    });
}

function recordRecentMedia(item: MediaItem) {
  if (item.sourceKind !== "localFilePath" || !item.path) {
    return;
  }

  runStorageCommand<RecentMediaDto>("storage_recent_media_record", { path: item.path, name: item.name })
    .then(() => refreshRecentMedia())
    .catch((error: unknown) => {
      console.error("Recent media record failed", error);
    });
}
```

Update the existing `useEffect` that reacts to `media?.id` so it records recent media and resets resume/progress refs:

```ts
useEffect(() => {
  if (!media) {
    return;
  }

  resumedMediaIdRef.current = null;
  lastProgressSaveRef.current = null;
  setCurrentTime(0);
  setDuration(0);
  setIsPlaying(false);
  setPlaybackError(null);
  mirrorOpenMedia(media);
  recordRecentMedia(media);
}, [media?.id]);
```

- [ ] **Step 5: Add progress save, clear, and resume helpers**

Add these functions before `handleCanPlay`:

```ts
function isResumePositionValid(progress: PlaybackProgressDto, durationMs: number | null) {
  if (progress.positionMs <= 10_000) {
    return false;
  }
  if (durationMs !== null && progress.positionMs >= durationMs - 10_000) {
    return false;
  }
  if (durationMs !== null && progress.positionMs >= durationMs) {
    return false;
  }
  return true;
}

function maybeResumePlayback(item: MediaItem, video: HTMLVideoElement, durationSeconds: number) {
  if (item.sourceKind !== "localFilePath" || !item.path || resumedMediaIdRef.current === item.id) {
    return;
  }

  resumedMediaIdRef.current = item.id;
  const durationMs = Number.isFinite(durationSeconds) && durationSeconds > 0 ? Math.round(durationSeconds * 1000) : null;

  runStorageCommand<PlaybackProgressDto | null>("storage_progress_get", { path: item.path })
    .then((savedProgress) => {
      if (!savedProgress || currentMediaIdRef.current !== item.id || videoRef.current !== video) {
        return;
      }
      if (!isResumePositionValid(savedProgress, durationMs)) {
        return;
      }

      const resumeSeconds = savedProgress.positionMs / 1000;
      video.currentTime = resumeSeconds;
      setCurrentTime(resumeSeconds);
      mirrorPlaybackCommand("playback_seek", { positionMs: savedProgress.positionMs });
    })
    .catch((error: unknown) => {
      console.error("Playback progress load failed", error);
    });
}

function maybeSavePlaybackProgress(positionSeconds: number, durationSeconds: number, force = false) {
  if (!media?.path || media.sourceKind !== "localFilePath" || !Number.isFinite(positionSeconds)) {
    return;
  }

  const positionMs = Math.max(0, Math.round(positionSeconds * 1000));
  const durationMs = Number.isFinite(durationSeconds) && durationSeconds > 0 ? Math.round(durationSeconds * 1000) : null;
  const lastSave = lastProgressSaveRef.current;
  if (!force && lastSave?.mediaId === media.id && Math.abs(positionMs - lastSave.positionMs) < 5_000) {
    return;
  }

  lastProgressSaveRef.current = { mediaId: media.id, positionMs };
  runStorageCommand<void>("storage_progress_save", { path: media.path, positionMs, durationMs }).catch((error: unknown) => {
    console.error("Playback progress save failed", error);
  });
}

function clearSavedPlaybackProgress(item: MediaItem | null) {
  if (!item?.path || item.sourceKind !== "localFilePath") {
    return;
  }

  runStorageCommand<void>("storage_progress_clear", { path: item.path }).catch((error: unknown) => {
    console.error("Playback progress clear failed", error);
  });
}
```

- [ ] **Step 6: Wire metadata, timeupdate, seek, and ended handlers**

Add this function before `togglePlayback`:

```ts
function handleLoadedMetadata(event: SyntheticEvent<HTMLVideoElement>) {
  event.currentTarget.volume = volumeLevel;
  setDuration(event.currentTarget.duration);
  if (media) {
    maybeResumePlayback(media, event.currentTarget, event.currentTarget.duration);
  }
}
```

Add this function before `togglePlayback`:

```ts
function handleTimeUpdate(event: SyntheticEvent<HTMLVideoElement>) {
  const nextTime = event.currentTarget.currentTime;
  setCurrentTime(nextTime);
  maybeSavePlaybackProgress(nextTime, event.currentTarget.duration);
}
```

Update `commitSeekTo` to force-save progress after user-initiated seek:

```ts
function commitSeekTo(value: number) {
  seekTo(value);
  if (Number.isFinite(value)) {
    mirrorPlaybackCommand("playback_seek", { positionMs: Math.round(value * 1000) });
    maybeSavePlaybackProgress(value, videoRef.current?.duration ?? duration, true);
  }
}
```

Update the `<video>` props:

```tsx
onLoadedMetadata={handleLoadedMetadata}
onTimeUpdate={handleTimeUpdate}
```

Update the existing `onEnded` handler to clear progress before auto-advance or stop:

```tsx
onEnded={() => {
  setIsPlaying(false);
  clearSavedPlaybackProgress(media);
  if (!advanceToNextQueueItem()) {
    mirrorPlaybackCommand("playback_stop");
  }
}}
```

- [ ] **Step 7: Add recent media shortcuts to empty state and playlist drawer**

Add this function before `openFiles`:

```ts
function openRecentMedia(item: RecentMediaDto) {
  setPlaybackError(null);
  replaceQueue([mediaItemFromNativePath(item.path, 0, item.name)]);
}
```

Inside the empty state, replace:

```tsx
<div className="empty-open">
  <span>Open media</span>
  <small>or drop a file anywhere</small>
</div>
```

with:

```tsx
<div className="empty-open">
  <span>Open media</span>
  <small>or drop a file anywhere</small>
  {recentMedia.length > 0 && (
    <div className="recent-shortcuts" aria-label="Recent media">
      {recentMedia.slice(0, 4).map((item) => (
        <button key={item.path} type="button" onClick={() => openRecentMedia(item)}>
          {item.name}
        </button>
      ))}
    </div>
  )}
</div>
```

In the playlist drawer, after the queue `<ol>...</ol>`, add:

```tsx
{recentMedia.length > 0 && (
  <div className="recent-drawer-section" aria-label="Recent media">
    <span>Recent</span>
    {recentMedia.map((item) => (
      <button key={item.path} type="button" onClick={() => openRecentMedia(item)}>
        {item.name}
      </button>
    ))}
  </div>
)}
```

- [ ] **Step 8: Add recent shortcut styles**

In `apps/desktop/src/styles.css`, update `.empty-open` to include `z-index: 6;` while keeping `pointer-events: none;`:

```css
.empty-open {
  position: absolute;
  inset: 0;
  z-index: 6;
  display: grid;
  place-content: center;
  background: transparent;
  color: var(--text);
  gap: 8px;
  pointer-events: none;
  text-align: center;
}
```

Add these styles before the media query:

```css
.recent-shortcuts {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  max-width: min(560px, calc(100vw - 48px));
  gap: 8px;
  margin-top: 18px;
  pointer-events: auto;
}

.recent-shortcuts button,
.recent-drawer-section button {
  overflow: hidden;
  border: 0;
  border-radius: 8px;
  background: rgba(236, 231, 221, 0.08);
  color: var(--muted);
  cursor: pointer;
  padding: 8px 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-shortcuts button:hover,
.recent-drawer-section button:hover {
  background: rgba(236, 231, 221, 0.14);
  color: var(--text);
}

.recent-drawer-section {
  display: grid;
  gap: 4px;
  border-top: 1px solid var(--line);
  margin-top: 8px;
  padding-top: 8px;
}

.recent-drawer-section > span {
  color: var(--faint);
  font-size: 0.72rem;
  padding: 0 2px 4px;
  text-transform: uppercase;
}

.recent-drawer-section button {
  width: 100%;
  text-align: left;
}
```

- [ ] **Step 9: Run frontend verification and build**

Run:

```powershell
npm run verify:shell
npm run build
```

Working directory: `apps/desktop`

Expected: both PASS.

- [ ] **Step 10: Commit frontend storage wiring**

Run:

```powershell
git add apps/desktop/src/App.tsx apps/desktop/src/styles.css apps/desktop/scripts/verify-shell.mjs
git commit -m "feat: persist recent media and playback progress"
```

Expected: commit succeeds and contains only frontend storage wiring and shell verification changes.

## Task 4: Final Verification

**Files:**
- No code changes expected.

- [ ] **Step 1: Run Rust verification**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Working directory: repository root

Expected: all PASS.

- [ ] **Step 2: Run frontend verification**

Run:

```powershell
npm run verify:shell
npm run build
```

Working directory: `apps/desktop`

Expected: both PASS.

- [ ] **Step 3: Inspect final status and recent diff**

Run:

```powershell
git status --short --branch
git diff --stat HEAD~3..HEAD
```

Working directory: repository root

Expected: working tree is clean if task commits were created. Recent commits cover storage repositories, desktop storage commands, and frontend storage wiring.

## Self-Review

- Spec coverage: SQLite foundation, migrations, recent media, playback progress, settings, Tauri commands, recent UI shortcuts, auto-resume, progress save/clear, and final verification are covered.
- Placeholder scan: no `TBD`, `TODO`, unspecified tests, or vague implementation steps remain.
- Type consistency: `RecentMedia`, `PlaybackProgress`, `SettingValue`, `RecentMediaDto`, `PlaybackProgressDto`, `DesktopStorageState`, `runStorageCommand`, `maybeResumePlayback`, and command names are used consistently.
- Scope check: browser drag/drop preview persistence, folders, HTTP URLs, full playlists, clear-history UI, and settings page remain out of scope.
