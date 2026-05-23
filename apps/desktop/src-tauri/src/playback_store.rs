use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const HISTORY_BY_PATH: TableDefinition<&str, &str> = TableDefinition::new("history_by_path");
const HISTORY_BY_UPDATED: TableDefinition<&str, &str> = TableDefinition::new("history_by_updated");
const HISTORY_LIMIT: usize = 10_000;
const HISTORY_LIST_LIMIT: usize = 100;
const MIN_RESUME_PROGRESS_RATIO: f64 = 0.01;
const RESUME_END_PROGRESS_RATIO: f64 = 0.95;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackHistoryEntry {
    path: String,
    name: String,
    position: f64,
    duration: f64,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackHistoryUpdate {
    path: String,
    name: Option<String>,
    position: f64,
    duration: f64,
    updated_at: Option<i64>,
}

pub struct PlaybackStoreState {
    path: PathBuf,
    access: Mutex<()>,
}

struct PlaybackStore {
    database: Database,
}

impl PlaybackStoreState {
    pub fn open(app: &AppHandle) -> Self {
        let path = match Self::store_path(app) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("{error}");
                PathBuf::from("playback-history.redb")
            }
        };

        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
        let mut directory = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
        directory.push("storage");
        Ok(directory.join("playback-history.redb"))
    }

    fn with_store<T>(
        &self,
        action: impl FnOnce(&mut PlaybackStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let _guard = self
            .access
            .lock()
            .map_err(|_| "playback history store lock failed".to_string())?;
        let mut store = PlaybackStore::open(self.path.clone())?;
        action(&mut store)
    }
}

impl PlaybackStore {
    fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create playback history directory: {error}"))?;
        }

        let database = create_database_with_retry(&path, "playback history")?;
        let store = Self { database };
        store.initialize()?;
        Ok(store)
    }

    fn initialize(&self) -> Result<(), String> {
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to initialize playback history: {error}"))?;
        {
            transaction
                .open_table(HISTORY_BY_PATH)
                .map_err(|error| format!("failed to open playback history table: {error}"))?;
            transaction
                .open_table(HISTORY_BY_UPDATED)
                .map_err(|error| format!("failed to open playback history index: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit playback history initialization: {error}"))
    }

    fn list(&self) -> Result<Vec<PlaybackHistoryEntry>, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read playback history: {error}"))?;
        let by_path = transaction
            .open_table(HISTORY_BY_PATH)
            .map_err(|error| format!("failed to open playback history table: {error}"))?;
        let by_updated = transaction
            .open_table(HISTORY_BY_UPDATED)
            .map_err(|error| format!("failed to open playback history index: {error}"))?;
        let mut entries = Vec::new();

        for item in by_updated
            .iter()
            .map_err(|error| format!("failed to scan playback history index: {error}"))?
            .take(HISTORY_LIST_LIMIT)
        {
            let (_, path) =
                item.map_err(|error| format!("failed to read playback history index: {error}"))?;
            if let Some(stored) = by_path
                .get(path.value())
                .map_err(|error| format!("failed to read playback history entry: {error}"))?
            {
                entries.push(decode_entry(stored.value())?);
            }
        }

        Ok(entries)
    }

    fn remember(
        &mut self,
        update: PlaybackHistoryUpdate,
    ) -> Result<Vec<PlaybackHistoryEntry>, String> {
        let entry = normalize_update(update)?;
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write playback history: {error}"))?;
        {
            let mut by_path = transaction
                .open_table(HISTORY_BY_PATH)
                .map_err(|error| format!("failed to open playback history table: {error}"))?;
            let mut by_updated = transaction
                .open_table(HISTORY_BY_UPDATED)
                .map_err(|error| format!("failed to open playback history index: {error}"))?;

            if let Some(previous) = by_path
                .get(entry.path.as_str())
                .map_err(|error| format!("failed to read old playback history entry: {error}"))?
            {
                let previous = decode_entry(previous.value())?;
                let old_index_key = updated_index_key(previous.updated_at, &previous.path);
                by_updated.remove(old_index_key.as_str()).map_err(|error| {
                    format!("failed to replace playback history index: {error}")
                })?;
            }

            let encoded = serde_json::to_string(&entry)
                .map_err(|error| format!("failed to encode playback history entry: {error}"))?;
            let index_key = updated_index_key(entry.updated_at, &entry.path);
            by_path
                .insert(entry.path.as_str(), encoded.as_str())
                .map_err(|error| format!("failed to store playback history entry: {error}"))?;
            by_updated
                .insert(index_key.as_str(), entry.path.as_str())
                .map_err(|error| format!("failed to store playback history index: {error}"))?;

            prune_history(&mut by_path, &mut by_updated)?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit playback history: {error}"))?;

        self.list()
    }

    fn resume_position(&self, path: &str) -> Result<f64, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read playback history: {error}"))?;
        let by_path = transaction
            .open_table(HISTORY_BY_PATH)
            .map_err(|error| format!("failed to open playback history table: {error}"))?;
        let Some(stored) = by_path
            .get(path.trim())
            .map_err(|error| format!("failed to read playback history entry: {error}"))?
        else {
            return Ok(0.0);
        };
        let entry = decode_entry(stored.value())?;
        Ok(resume_position_for_entry(entry.position, entry.duration))
    }

    fn clear(&mut self) -> Result<Vec<PlaybackHistoryEntry>, String> {
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to clear playback history: {error}"))?;
        {
            let mut by_path = transaction
                .open_table(HISTORY_BY_PATH)
                .map_err(|error| format!("failed to open playback history table: {error}"))?;
            let mut by_updated = transaction
                .open_table(HISTORY_BY_UPDATED)
                .map_err(|error| format!("failed to open playback history index: {error}"))?;

            let path_keys = table_keys(&by_path, "playback history entries")?;
            for key in path_keys {
                by_path
                    .remove(key.as_str())
                    .map_err(|error| format!("failed to remove playback history entry: {error}"))?;
            }

            let updated_keys = table_keys(&by_updated, "playback history index")?;
            for key in updated_keys {
                by_updated
                    .remove(key.as_str())
                    .map_err(|error| format!("failed to remove playback history index: {error}"))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit playback history clear: {error}"))?;

        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn history_list(
    state: State<'_, PlaybackStoreState>,
) -> Result<Vec<PlaybackHistoryEntry>, String> {
    state.with_store(|store| store.list())
}

#[tauri::command]
pub fn history_remember(
    state: State<'_, PlaybackStoreState>,
    entry: PlaybackHistoryUpdate,
) -> Result<Vec<PlaybackHistoryEntry>, String> {
    state.with_store(|store| store.remember(entry))
}

#[tauri::command]
pub fn history_resume_position(
    state: State<'_, PlaybackStoreState>,
    path: String,
) -> Result<f64, String> {
    state.with_store(|store| store.resume_position(&path))
}

#[tauri::command]
pub fn history_clear(
    state: State<'_, PlaybackStoreState>,
) -> Result<Vec<PlaybackHistoryEntry>, String> {
    state.with_store(|store| store.clear())
}

fn create_database_with_retry(path: &Path, label: &str) -> Result<Database, String> {
    let mut last_error = None;
    for _ in 0..16 {
        match Database::create(path) {
            Ok(database) => return Ok(database),
            Err(error) => {
                last_error = Some(error.to_string());
                thread::sleep(Duration::from_millis(20));
            }
        }
    }

    Err(format!(
        "failed to open {label} database: {}",
        last_error.unwrap_or_else(|| "unknown redb error".to_string())
    ))
}

fn table_keys<T>(table: &T, label: &str) -> Result<Vec<String>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    table
        .iter()
        .map_err(|error| format!("failed to scan {label}: {error}"))?
        .map(|item| {
            item.map(|(key, _)| key.value().to_string())
                .map_err(|error| format!("failed to read {label}: {error}"))
        })
        .collect()
}

fn prune_history(
    by_path: &mut redb::Table<'_, &str, &str>,
    by_updated: &mut redb::Table<'_, &str, &str>,
) -> Result<(), String> {
    while by_updated
        .len()
        .map_err(|error| format!("failed to count playback history entries: {error}"))?
        > HISTORY_LIMIT as u64
    {
        let oldest = by_updated
            .last()
            .map_err(|error| format!("failed to find old playback history entry: {error}"))?
            .map(|(index_key, path)| (index_key.value().to_string(), path.value().to_string()));
        let Some((index_key, path)) = oldest else {
            break;
        };
        by_updated
            .remove(index_key.as_str())
            .map_err(|error| format!("failed to prune playback history index: {error}"))?;
        by_path
            .remove(path.as_str())
            .map_err(|error| format!("failed to prune playback history entry: {error}"))?;
    }

    Ok(())
}

fn normalize_update(update: PlaybackHistoryUpdate) -> Result<PlaybackHistoryEntry, String> {
    let path = update.path.trim().to_string();
    if path.is_empty() {
        return Err("playback history path is empty".to_string());
    }

    Ok(PlaybackHistoryEntry {
        name: update
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| media_name_from_path(&path)),
        path,
        position: normalize_non_negative_number(update.position),
        duration: normalize_non_negative_number(update.duration),
        updated_at: update.updated_at.unwrap_or_else(now_millis).max(0),
    })
}

fn decode_entry(value: &str) -> Result<PlaybackHistoryEntry, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode playback history entry: {error}"))
}

fn normalize_non_negative_number(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn resume_position_for_entry(position: f64, duration: f64) -> f64 {
    if !position.is_finite() || !duration.is_finite() || duration <= 0.0 || position <= 0.0 {
        return 0.0;
    }

    let clamped = position.clamp(0.0, duration);
    let ratio = clamped / duration;
    if !(MIN_RESUME_PROGRESS_RATIO..RESUME_END_PROGRESS_RATIO).contains(&ratio) {
        0.0
    } else {
        clamped
    }
}

fn updated_index_key(updated_at: i64, path: &str) -> String {
    let newest_first = u64::MAX - updated_at.max(0) as u64;
    format!("{newest_first:020}|{path}")
}

fn media_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_position_uses_duration_ratios() {
        assert_eq!(resume_position_for_entry(0.5, 2.0), 0.5);
        assert_eq!(resume_position_for_entry(2.0, 400.0), 0.0);
        assert_eq!(resume_position_for_entry(96.0, 100.0), 0.0);
    }

    #[test]
    fn redb_store_updates_existing_paths_and_lists_newest_first() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-history-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&directory).expect("temp history directory should be created");
        let database_path = directory.join("history.redb");
        let mut store = PlaybackStore::open(database_path).expect("redb store should open");

        store
            .remember(PlaybackHistoryUpdate {
                path: "E:\\Media\\first.mp4".to_string(),
                name: Some("first.mp4".to_string()),
                position: 40.0,
                duration: 100.0,
                updated_at: Some(10),
            })
            .expect("first entry should be written");
        store
            .remember(PlaybackHistoryUpdate {
                path: "E:\\Media\\second.mp4".to_string(),
                name: Some("second.mp4".to_string()),
                position: 80.0,
                duration: 100.0,
                updated_at: Some(20),
            })
            .expect("second entry should be written");
        store
            .remember(PlaybackHistoryUpdate {
                path: "E:\\Media\\first.mp4".to_string(),
                name: Some("first.mp4".to_string()),
                position: 50.0,
                duration: 100.0,
                updated_at: Some(30),
            })
            .expect("first entry should be updated");

        let entries = store.list().expect("history should be readable");
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "E:\\Media\\first.mp4");
        assert_eq!(entries[0].position, 50.0);
        assert_eq!(entries[1].path, "E:\\Media\\second.mp4");
    }

    #[test]
    fn redb_store_clears_history_and_resume_positions() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-history-clear-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&directory).expect("temp history directory should be created");
        let database_path = directory.join("history.redb");
        let mut store = PlaybackStore::open(database_path).expect("redb store should open");

        store
            .remember(PlaybackHistoryUpdate {
                path: "E:\\Media\\first.mp4".to_string(),
                name: Some("first.mp4".to_string()),
                position: 40.0,
                duration: 100.0,
                updated_at: Some(10),
            })
            .expect("entry should be written");

        let entries = store.clear().expect("history should clear");
        let resume = store
            .resume_position("E:\\Media\\first.mp4")
            .expect("resume lookup should still work");
        let _ = fs::remove_dir_all(&directory);

        assert!(entries.is_empty());
        assert_eq!(resume, 0.0);
    }
}
