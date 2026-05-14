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
            connection
                .execute_batch("PRAGMA foreign_keys = ON;")
                .map_err(StorageError::from)?;
            for migration in MIGRATIONS {
                connection
                    .execute_batch(migration)
                    .map_err(StorageError::from)?;
            }
            Ok(())
        })
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::LockFailed)?;
        operation(&connection)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RecentMediaRepository<'a> {
    database: &'a StorageDatabase,
}

impl RecentMediaRepository<'_> {
    pub fn record(&self, path: &str, name: &str, now_ms: i64) -> Result<(), StorageError> {
        self.record_and_get(path, name, now_ms).map(|_| ())
    }

    pub fn record_and_get(
        &self,
        path: &str,
        name: &str,
        now_ms: i64,
    ) -> Result<RecentMedia, StorageError> {
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
            Self::get_with_connection(connection, path).and_then(|item| {
                item.ok_or_else(|| {
                    StorageError::QueryFailed("recent media row missing".to_string())
                })
            })
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

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(StorageError::from)
        })
    }

    pub fn get(&self, path: &str) -> Result<Option<RecentMedia>, StorageError> {
        validate_non_empty(path, "path")?;
        self.database
            .with_connection(|connection| Self::get_with_connection(connection, path))
    }

    fn get_with_connection(
        connection: &Connection,
        path: &str,
    ) -> Result<Option<RecentMedia>, StorageError> {
        connection
            .query_row(
                "SELECT path, name, last_opened_at_ms, open_count
                 FROM recent_media
                 WHERE path = ?1",
                params![path],
                |row| {
                    Ok(RecentMedia {
                        path: row.get(0)?,
                        name: row.get(1)?,
                        last_opened_at_ms: row.get(2)?,
                        open_count: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(StorageError::from)
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
                .execute(
                    "DELETE FROM playback_progress WHERE path = ?1",
                    params![path],
                )
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

        assert!(
            database
                .table_exists("recent_media")
                .expect("recent_media table")
        );
        assert!(
            database
                .table_exists("playback_progress")
                .expect("playback_progress table")
        );
        assert!(database.table_exists("settings").expect("settings table"));
    }

    #[test]
    fn recent_media_upsert_increments_count_and_orders_by_last_opened() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();

        recent
            .record("C:/media/a.mp4", "a.mp4", 100)
            .expect("record a");
        recent
            .record("C:/media/b.mp4", "b.mp4", 200)
            .expect("record b");
        recent
            .record("C:/media/a.mp4", "a-renamed.mp4", 300)
            .expect("record a again");

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

        recent
            .record("C:/media/a.mp4", "a.mp4", 100)
            .expect("record a");
        recent
            .record("C:/media/b.mp4", "b.mp4", 200)
            .expect("record b");

        let items = recent.list(1).expect("recent list");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "C:/media/b.mp4");
    }

    #[test]
    fn recent_media_get_finds_item_outside_list_limit() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();
        let target_path = "C:/media/target.mp4";

        recent
            .record(target_path, "target.mp4", 1)
            .expect("record target");
        for index in 0..101 {
            recent
                .record(
                    &format!("C:/media/newer-{index}.mp4"),
                    &format!("newer-{index}.mp4"),
                    1_000 + index,
                )
                .expect("record newer item");
        }

        let listed = recent.list(100).expect("recent list");
        let found = recent
            .get(target_path)
            .expect("get target")
            .expect("target row");

        assert!(!listed.iter().any(|item| item.path == target_path));
        assert_eq!(found.path, target_path);
        assert_eq!(found.name, "target.mp4");
        assert_eq!(found.last_opened_at_ms, 1);
    }

    #[test]
    fn recent_media_record_and_get_returns_updated_row() {
        let database = StorageDatabase::in_memory().expect("database");
        let recent = database.recent_media();

        recent
            .record_and_get("C:/media/a.mp4", "a.mp4", 100)
            .expect("record a");
        let updated = recent
            .record_and_get("C:/media/a.mp4", "a-renamed.mp4", 200)
            .expect("record a again");

        assert_eq!(updated.path, "C:/media/a.mp4");
        assert_eq!(updated.name, "a-renamed.mp4");
        assert_eq!(updated.last_opened_at_ms, 200);
        assert_eq!(updated.open_count, 2);
    }

    #[test]
    fn playback_progress_roundtrip_and_clear() {
        let database = StorageDatabase::in_memory().expect("database");
        let progress = database.playback_progress();

        progress
            .save("C:/media/a.mp4", 42_000, Some(120_000), 500)
            .expect("save progress");
        let saved = progress
            .get("C:/media/a.mp4")
            .expect("get progress")
            .expect("saved progress");

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

        settings
            .set("theme", "studio-dark", 700)
            .expect("set setting");
        let value = settings
            .get("theme")
            .expect("get setting")
            .expect("setting");

        assert_eq!(value.key, "theme");
        assert_eq!(value.value, "studio-dark");
        assert_eq!(value.updated_at_ms, 700);
    }

    #[test]
    fn repositories_reject_empty_inputs() {
        let database = StorageDatabase::in_memory().expect("database");

        assert_eq!(
            database.recent_media().record("", "movie.mp4", 1),
            Err(StorageError::InvalidInput("path"))
        );
        assert_eq!(
            database.recent_media().record("C:/media/a.mp4", "", 1),
            Err(StorageError::InvalidInput("name"))
        );
        assert_eq!(
            database.playback_progress().save("", 1, None, 1),
            Err(StorageError::InvalidInput("path"))
        );
        assert_eq!(
            database.settings().set("", "value", 1),
            Err(StorageError::InvalidInput("key"))
        );
    }
}
