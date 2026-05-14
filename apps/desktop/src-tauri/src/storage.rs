use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use openplayer_storage::{PlaybackProgress, RecentMedia, StorageDatabase, StorageError};
use serde::{Deserialize, Serialize};
use tauri::State;

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

    #[cfg(test)]
    pub fn in_memory_for_tests() -> Self {
        match StorageDatabase::in_memory() {
            Ok(database) => Self {
                database: Some(Arc::new(database)),
                init_error: None,
            },
            Err(error) => Self::unavailable(error.to_string()),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            database: None,
            init_error: Some(message.into()),
        }
    }

    pub fn list_recent_media(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<RecentMediaDto>, StorageCommandError> {
        let database = self.database()?;
        database
            .recent_media()
            .list(limit.unwrap_or(20))
            .map(|items| items.into_iter().map(RecentMediaDto::from).collect())
            .map_err(StorageCommandError::from)
    }

    pub fn record_recent_media(
        &self,
        path: String,
        name: String,
        opened_at_ms: Option<i64>,
    ) -> Result<RecentMediaDto, StorageCommandError> {
        let database = self.database()?;
        let opened_at_ms = match opened_at_ms {
            Some(value) if value < 0 => {
                return Err(StorageCommandError::new(
                    "storage.invalidInput",
                    "Storage input is invalid",
                ));
            }
            Some(value) => value,
            None => now_millis()?,
        };
        database
            .recent_media()
            .record_and_get(&path, &name, opened_at_ms)
            .map(RecentMediaDto::from)
            .map_err(StorageCommandError::from)
    }

    pub fn get_progress(
        &self,
        path: String,
    ) -> Result<Option<PlaybackProgressDto>, StorageCommandError> {
        self.database()?
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
        self.database()?
            .playback_progress()
            .save(&path, position_ms, duration_ms, now_millis()?)
            .map_err(StorageCommandError::from)
    }

    pub fn clear_progress(&self, path: String) -> Result<(), StorageCommandError> {
        self.database()?
            .playback_progress()
            .clear(&path)
            .map_err(StorageCommandError::from)
    }

    pub fn get_setting(&self, key: String) -> Result<Option<String>, StorageCommandError> {
        self.database()?
            .settings()
            .get(&key)
            .map(|setting| setting.map(|setting| setting.value))
            .map_err(StorageCommandError::from)
    }

    pub fn set_setting(&self, key: String, value: String) -> Result<(), StorageCommandError> {
        self.database()?
            .settings()
            .set(&key, &value, now_millis()?)
            .map_err(StorageCommandError::from)
    }

    fn database(&self) -> Result<&StorageDatabase, StorageCommandError> {
        self.database.as_deref().ok_or_else(|| {
            StorageCommandError::new(
                "storage.unavailable",
                if self.init_error.is_some() {
                    "Storage is unavailable"
                } else {
                    "Storage is not configured"
                },
            )
        })
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

    fn query_failed(message: impl Into<String>) -> Self {
        Self::new("storage.queryFailed", message)
    }
}

impl From<StorageError> for StorageCommandError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::NotConfigured => {
                Self::new("storage.unavailable", "Storage is not configured")
            }
            StorageError::InvalidInput(_) => {
                Self::new("storage.invalidInput", "Storage input is invalid")
            }
            StorageError::QueryFailed(_) | StorageError::LockFailed => {
                Self::query_failed("Storage query failed")
            }
        }
    }
}

fn now_millis() -> Result<i64, StorageCommandError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StorageCommandError::new("storage.invalidClock", "System clock is invalid"))?;
    Ok(i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
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
    opened_at_ms: Option<i64>,
) -> Result<RecentMediaDto, StorageCommandError> {
    state.record_recent_media(path, name, opened_at_ms)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_media_commands_record_and_list_items() {
        let state = DesktopStorageState::in_memory_for_tests();

        let recorded = state
            .record_recent_media(
                "C:/media/movie.mp4".to_string(),
                "movie.mp4".to_string(),
                None,
            )
            .expect("record recent media");
        let items = state
            .list_recent_media(Some(10))
            .expect("list recent media");

        assert_eq!(recorded.path, "C:/media/movie.mp4");
        assert_eq!(recorded.name, "movie.mp4");
        assert_eq!(recorded.open_count, 1);
        assert_eq!(items, vec![recorded]);
    }

    #[test]
    fn recent_media_uses_frontend_open_time_for_ordering() {
        let state = DesktopStorageState::in_memory_for_tests();

        state
            .record_recent_media(
                "C:/media/a.mp4".to_string(),
                "a.mp4".to_string(),
                Some(2_000),
            )
            .expect("record a");
        state
            .record_recent_media(
                "C:/media/b.mp4".to_string(),
                "b.mp4".to_string(),
                Some(3_000),
            )
            .expect("record b");
        state
            .record_recent_media(
                "C:/media/a.mp4".to_string(),
                "a.mp4".to_string(),
                Some(2_000),
            )
            .expect("late duplicate a");

        let items = state
            .list_recent_media(Some(10))
            .expect("list recent media");

        assert_eq!(items[0].path, "C:/media/b.mp4");
        assert_eq!(items[0].last_opened_at_ms, 3_000);
        assert_eq!(items[1].path, "C:/media/a.mp4");
        assert_eq!(items[1].last_opened_at_ms, 2_000);
    }

    #[test]
    fn progress_commands_save_get_and_clear() {
        let state = DesktopStorageState::in_memory_for_tests();
        let path = "C:/media/movie.mp4".to_string();

        state
            .save_progress(path.clone(), 42_000, Some(120_000))
            .expect("save progress");
        let progress = state
            .get_progress(path.clone())
            .expect("get progress")
            .expect("saved progress");

        assert_eq!(progress.path, path);
        assert_eq!(progress.position_ms, 42_000);
        assert_eq!(progress.duration_ms, Some(120_000));

        state.clear_progress(path.clone()).expect("clear progress");
        assert_eq!(state.get_progress(path).expect("get cleared"), None);
    }

    #[test]
    fn setting_commands_roundtrip() {
        let state = DesktopStorageState::in_memory_for_tests();

        state
            .set_setting("theme".to_string(), "studio-dark".to_string())
            .expect("set setting");

        assert_eq!(
            state.get_setting("theme".to_string()).expect("get setting"),
            Some("studio-dark".to_string())
        );
    }

    #[test]
    fn unavailable_storage_maps_to_stable_error() {
        let state = DesktopStorageState::unavailable("database failed");

        let error = state
            .list_recent_media(None)
            .expect_err("unavailable storage");

        assert_eq!(error.code, "storage.unavailable");
        assert_eq!(error.message, "Storage is unavailable");
    }

    #[test]
    fn invalid_input_maps_to_stable_error() {
        let state = DesktopStorageState::in_memory_for_tests();

        let error = state
            .record_recent_media("".to_string(), "movie.mp4".to_string(), None)
            .expect_err("invalid path");

        assert_eq!(error.code, "storage.invalidInput");
        assert_eq!(error.message, "Storage input is invalid");
    }

    #[test]
    fn negative_recent_media_open_time_maps_to_stable_error() {
        let state = DesktopStorageState::in_memory_for_tests();

        let error = state
            .record_recent_media(
                "C:/media/movie.mp4".to_string(),
                "movie.mp4".to_string(),
                Some(-1),
            )
            .expect_err("invalid open time");

        assert_eq!(error.code, "storage.invalidInput");
        assert_eq!(error.message, "Storage input is invalid");
    }
}
