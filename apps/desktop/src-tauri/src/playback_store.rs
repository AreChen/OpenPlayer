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
const PLAYBACK_SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("playback_settings");
const MEDIA_SETTINGS_BY_PATH: TableDefinition<&str, &str> =
    TableDefinition::new("media_settings_by_path");
const NETWORK_STREAMS_BY_URL: TableDefinition<&str, &str> =
    TableDefinition::new("network_streams_by_url");
const NETWORK_STREAMS_BY_UPDATED: TableDefinition<&str, &str> =
    TableDefinition::new("network_streams_by_updated");
const HISTORY_LIMIT: usize = 10_000;
const HISTORY_LIST_LIMIT: usize = 100;
const NETWORK_STREAM_HISTORY_LIMIT: usize = 500;
const NETWORK_STREAM_HISTORY_LIST_LIMIT: usize = 50;
const MIN_RESUME_PROGRESS_RATIO: f64 = 0.01;
const RESUME_END_PROGRESS_RATIO: f64 = 0.95;
const PLAYBACK_SETTINGS_KEY: &str = "global";
const DEFAULT_VOLUME: f64 = 82.0;
const DEFAULT_LOOP_MODE: &str = "off";
const DEFAULT_HWDEC_MODE: &str = "hardware";
const DEFAULT_PLAYBACK_SPEED: f64 = 1.0;
const DEFAULT_TIME_DISPLAY_MODE: &str = "timecode";
const MIN_PLAYBACK_SPEED: f64 = 0.25;
const MAX_PLAYBACK_SPEED: f64 = 4.0;

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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettings {
    volume: f64,
    loop_mode: String,
    hwdec_mode: String,
    playback_speed: f64,
    video_fill: bool,
    time_display_mode: String,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            volume: DEFAULT_VOLUME,
            loop_mode: DEFAULT_LOOP_MODE.to_string(),
            hwdec_mode: DEFAULT_HWDEC_MODE.to_string(),
            playback_speed: DEFAULT_PLAYBACK_SPEED,
            video_fill: false,
            time_display_mode: DEFAULT_TIME_DISPLAY_MODE.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettingsUpdate {
    volume: Option<f64>,
    loop_mode: Option<String>,
    hwdec_mode: Option<String>,
    playback_speed: Option<f64>,
    video_fill: Option<bool>,
    time_display_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaybackSettings {
    path: String,
    subtitle_track_id: Option<i64>,
    has_subtitle_track_selection: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaybackSettingsUpdate {
    #[serde(default)]
    subtitle_track_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStreamHistoryEntry {
    url: String,
    name: String,
    scheme: String,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStreamHistoryUpdate {
    url: String,
    name: Option<String>,
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
            transaction
                .open_table(PLAYBACK_SETTINGS)
                .map_err(|error| format!("failed to open playback settings table: {error}"))?;
            transaction
                .open_table(MEDIA_SETTINGS_BY_PATH)
                .map_err(|error| {
                    format!("failed to open media playback settings table: {error}")
                })?;
            transaction
                .open_table(NETWORK_STREAMS_BY_URL)
                .map_err(|error| format!("failed to open network stream history table: {error}"))?;
            transaction
                .open_table(NETWORK_STREAMS_BY_UPDATED)
                .map_err(|error| format!("failed to open network stream history index: {error}"))?;
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
        let entry_key = store_key_for_path(&entry.path);
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

            for old_key in existing_history_keys(&by_path, &entry_key, &entry.path)? {
                if let Some(previous) = by_path.get(old_key.as_str()).map_err(|error| {
                    format!("failed to read old playback history entry: {error}")
                })? {
                    let previous = decode_entry(previous.value())?;
                    let old_index_key = updated_index_key(previous.updated_at, &old_key);
                    by_updated.remove(old_index_key.as_str()).map_err(|error| {
                        format!("failed to replace playback history index: {error}")
                    })?;
                }
                by_path.remove(old_key.as_str()).map_err(|error| {
                    format!("failed to replace playback history entry: {error}")
                })?;
            }

            let encoded = serde_json::to_string(&entry)
                .map_err(|error| format!("failed to encode playback history entry: {error}"))?;
            let index_key = updated_index_key(entry.updated_at, &entry_key);
            by_path
                .insert(entry_key.as_str(), encoded.as_str())
                .map_err(|error| format!("failed to store playback history entry: {error}"))?;
            by_updated
                .insert(index_key.as_str(), entry_key.as_str())
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
        let Some(stored) = get_by_normalized_or_legacy_key(&by_path, path)? else {
            return Ok(0.0);
        };
        let entry = decode_entry(stored.value())?;
        Ok(resume_position_for_entry(entry.position, entry.duration))
    }

    fn settings(&self) -> Result<PlaybackSettings, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read playback settings: {error}"))?;
        let table = transaction
            .open_table(PLAYBACK_SETTINGS)
            .map_err(|error| format!("failed to open playback settings table: {error}"))?;
        let Some(stored) = table
            .get(PLAYBACK_SETTINGS_KEY)
            .map_err(|error| format!("failed to read playback settings entry: {error}"))?
        else {
            return Ok(PlaybackSettings::default());
        };

        decode_settings(stored.value())
    }

    fn update_settings(
        &mut self,
        update: PlaybackSettingsUpdate,
    ) -> Result<PlaybackSettings, String> {
        let mut settings = self.settings()?;
        merge_settings_update(&mut settings, update);
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write playback settings: {error}"))?;
        {
            let mut table = transaction
                .open_table(PLAYBACK_SETTINGS)
                .map_err(|error| format!("failed to open playback settings table: {error}"))?;
            let encoded = serde_json::to_string(&settings)
                .map_err(|error| format!("failed to encode playback settings: {error}"))?;
            table
                .insert(PLAYBACK_SETTINGS_KEY, encoded.as_str())
                .map_err(|error| format!("failed to store playback settings: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit playback settings: {error}"))?;

        Ok(settings)
    }

    fn media_settings(&self, path: &str) -> Result<MediaPlaybackSettings, String> {
        let normalized_path = path.trim();
        let key = store_key_for_path(normalized_path);
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read media playback settings: {error}"))?;
        let table = transaction
            .open_table(MEDIA_SETTINGS_BY_PATH)
            .map_err(|error| format!("failed to open media playback settings table: {error}"))?;
        let Some(stored) = get_by_normalized_or_legacy_key(&table, normalized_path)? else {
            return Ok(MediaPlaybackSettings {
                path: normalized_path.to_string(),
                subtitle_track_id: None,
                has_subtitle_track_selection: false,
            });
        };
        let mut settings = decode_media_settings(stored.value())?;
        settings.path = normalized_path.to_string();
        if settings.path.is_empty() {
            settings.path = key;
        }
        Ok(settings)
    }

    fn update_media_settings(
        &mut self,
        path: &str,
        update: MediaPlaybackSettingsUpdate,
    ) -> Result<MediaPlaybackSettings, String> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err("media playback settings path is empty".to_string());
        }

        let key = store_key_for_path(trimmed);
        let mut settings = self.media_settings(trimmed)?;
        settings.path = trimmed.to_string();
        if let Some(subtitle_track_id) = update.subtitle_track_id {
            settings.subtitle_track_id = normalize_track_id(subtitle_track_id)?;
            settings.has_subtitle_track_selection = true;
        }

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write media playback settings: {error}"))?;
        {
            let mut table = transaction
                .open_table(MEDIA_SETTINGS_BY_PATH)
                .map_err(|error| {
                    format!("failed to open media playback settings table: {error}")
                })?;
            let legacy_key = trimmed.to_string();
            if legacy_key != key {
                let _ = table.remove(legacy_key.as_str());
            }
            let encoded = serde_json::to_string(&settings)
                .map_err(|error| format!("failed to encode media playback settings: {error}"))?;
            table
                .insert(key.as_str(), encoded.as_str())
                .map_err(|error| format!("failed to store media playback settings: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit media playback settings: {error}"))?;

        Ok(settings)
    }

    fn network_stream_history(&self) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read network stream history: {error}"))?;
        let by_url = transaction
            .open_table(NETWORK_STREAMS_BY_URL)
            .map_err(|error| format!("failed to open network stream history table: {error}"))?;
        let by_updated = transaction
            .open_table(NETWORK_STREAMS_BY_UPDATED)
            .map_err(|error| format!("failed to open network stream history index: {error}"))?;
        let mut entries = Vec::new();

        for item in by_updated
            .iter()
            .map_err(|error| format!("failed to scan network stream history index: {error}"))?
            .take(NETWORK_STREAM_HISTORY_LIST_LIMIT)
        {
            let (_, url_key) = item
                .map_err(|error| format!("failed to read network stream history index: {error}"))?;
            if let Some(stored) = by_url
                .get(url_key.value())
                .map_err(|error| format!("failed to read network stream history entry: {error}"))?
            {
                entries.push(decode_network_stream_entry(stored.value())?);
            }
        }

        Ok(entries)
    }

    fn remember_network_stream(
        &mut self,
        update: NetworkStreamHistoryUpdate,
    ) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
        let entry = normalize_network_stream_update(update)?;
        let entry_key = network_stream_key_for_url(&entry.url);
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write network stream history: {error}"))?;
        {
            let mut by_url = transaction
                .open_table(NETWORK_STREAMS_BY_URL)
                .map_err(|error| format!("failed to open network stream history table: {error}"))?;
            let mut by_updated = transaction
                .open_table(NETWORK_STREAMS_BY_UPDATED)
                .map_err(|error| format!("failed to open network stream history index: {error}"))?;

            if let Some(previous) = by_url
                .get(entry_key.as_str())
                .map_err(|error| format!("failed to read old network stream history: {error}"))?
            {
                let previous = decode_network_stream_entry(previous.value())?;
                let old_index_key = updated_index_key(previous.updated_at, &entry_key);
                by_updated.remove(old_index_key.as_str()).map_err(|error| {
                    format!("failed to replace network stream history index: {error}")
                })?;
            }

            let encoded = serde_json::to_string(&entry).map_err(|error| {
                format!("failed to encode network stream history entry: {error}")
            })?;
            let index_key = updated_index_key(entry.updated_at, &entry_key);
            by_url
                .insert(entry_key.as_str(), encoded.as_str())
                .map_err(|error| {
                    format!("failed to store network stream history entry: {error}")
                })?;
            by_updated
                .insert(index_key.as_str(), entry_key.as_str())
                .map_err(|error| {
                    format!("failed to store network stream history index: {error}")
                })?;

            prune_network_stream_history(&mut by_url, &mut by_updated)?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit network stream history: {error}"))?;

        self.network_stream_history()
    }

    fn clear_network_stream_history(&mut self) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to clear network stream history: {error}"))?;
        {
            let mut by_url = transaction
                .open_table(NETWORK_STREAMS_BY_URL)
                .map_err(|error| format!("failed to open network stream history table: {error}"))?;
            let mut by_updated = transaction
                .open_table(NETWORK_STREAMS_BY_UPDATED)
                .map_err(|error| format!("failed to open network stream history index: {error}"))?;

            for key in table_keys(&by_url, "network stream history entries")? {
                by_url.remove(key.as_str()).map_err(|error| {
                    format!("failed to remove network stream history entry: {error}")
                })?;
            }
            for key in table_keys(&by_updated, "network stream history index")? {
                by_updated.remove(key.as_str()).map_err(|error| {
                    format!("failed to remove network stream history index: {error}")
                })?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit network stream history clear: {error}"))?;

        Ok(Vec::new())
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

#[tauri::command]
pub fn playback_settings_state(
    state: State<'_, PlaybackStoreState>,
) -> Result<PlaybackSettings, String> {
    state.with_store(|store| store.settings())
}

#[tauri::command]
pub fn playback_settings_update(
    state: State<'_, PlaybackStoreState>,
    settings: PlaybackSettingsUpdate,
) -> Result<PlaybackSettings, String> {
    state.with_store(|store| store.update_settings(settings))
}

#[tauri::command]
pub fn playback_media_settings(
    state: State<'_, PlaybackStoreState>,
    path: String,
) -> Result<MediaPlaybackSettings, String> {
    state.with_store(|store| store.media_settings(&path))
}

#[tauri::command]
pub fn playback_media_settings_update(
    state: State<'_, PlaybackStoreState>,
    path: String,
    settings: MediaPlaybackSettingsUpdate,
) -> Result<MediaPlaybackSettings, String> {
    state.with_store(|store| store.update_media_settings(&path, settings))
}

#[tauri::command]
pub fn network_stream_history_list(
    state: State<'_, PlaybackStoreState>,
) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
    state.with_store(|store| store.network_stream_history())
}

#[tauri::command]
pub fn network_stream_history_remember(
    state: State<'_, PlaybackStoreState>,
    entry: NetworkStreamHistoryUpdate,
) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
    state.with_store(|store| store.remember_network_stream(entry))
}

#[tauri::command]
pub fn network_stream_history_clear(
    state: State<'_, PlaybackStoreState>,
) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
    state.with_store(|store| store.clear_network_stream_history())
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

fn prune_network_stream_history(
    by_url: &mut redb::Table<'_, &str, &str>,
    by_updated: &mut redb::Table<'_, &str, &str>,
) -> Result<(), String> {
    while by_updated
        .len()
        .map_err(|error| format!("failed to count network stream history entries: {error}"))?
        > NETWORK_STREAM_HISTORY_LIMIT as u64
    {
        let oldest = by_updated
            .last()
            .map_err(|error| format!("failed to find old network stream history entry: {error}"))?
            .map(|(index_key, url)| (index_key.value().to_string(), url.value().to_string()));
        let Some((index_key, url)) = oldest else {
            break;
        };
        by_updated
            .remove(index_key.as_str())
            .map_err(|error| format!("failed to prune network stream history index: {error}"))?;
        by_url
            .remove(url.as_str())
            .map_err(|error| format!("failed to prune network stream history entry: {error}"))?;
    }

    Ok(())
}

fn existing_history_keys(
    by_path: &redb::Table<'_, &str, &str>,
    normalized_key: &str,
    display_path: &str,
) -> Result<Vec<String>, String> {
    let mut keys = Vec::new();
    if by_path
        .get(normalized_key)
        .map_err(|error| format!("failed to read old playback history entry: {error}"))?
        .is_some()
    {
        keys.push(normalized_key.to_string());
    }

    let legacy_key = display_path.trim();
    if legacy_key != normalized_key
        && by_path
            .get(legacy_key)
            .map_err(|error| format!("failed to read old playback history entry: {error}"))?
            .is_some()
    {
        keys.push(legacy_key.to_string());
    }

    Ok(keys)
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

fn decode_settings(value: &str) -> Result<PlaybackSettings, String> {
    serde_json::from_str(value)
        .map(sanitize_playback_settings)
        .map_err(|error| format!("failed to decode playback settings: {error}"))
}

fn decode_media_settings(value: &str) -> Result<MediaPlaybackSettings, String> {
    serde_json::from_str(value)
        .map(sanitize_media_settings)
        .map_err(|error| format!("failed to decode media playback settings: {error}"))
}

fn decode_network_stream_entry(value: &str) -> Result<NetworkStreamHistoryEntry, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode network stream history entry: {error}"))
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

fn merge_settings_update(settings: &mut PlaybackSettings, update: PlaybackSettingsUpdate) {
    if let Some(volume) = update.volume {
        settings.volume = normalize_volume(volume);
    }
    if let Some(loop_mode) = update.loop_mode {
        settings.loop_mode = normalize_loop_mode(&loop_mode);
    }
    if let Some(hwdec_mode) = update.hwdec_mode {
        settings.hwdec_mode = normalize_hwdec_mode(&hwdec_mode);
    }
    if let Some(playback_speed) = update.playback_speed {
        settings.playback_speed = normalize_playback_speed(playback_speed);
    }
    if let Some(video_fill) = update.video_fill {
        settings.video_fill = video_fill;
    }
    if let Some(time_display_mode) = update.time_display_mode {
        settings.time_display_mode = normalize_time_display_mode(&time_display_mode);
    }
}

fn sanitize_playback_settings(mut settings: PlaybackSettings) -> PlaybackSettings {
    settings.volume = normalize_volume(settings.volume);
    settings.loop_mode = normalize_loop_mode(&settings.loop_mode);
    settings.hwdec_mode = normalize_hwdec_mode(&settings.hwdec_mode);
    settings.playback_speed = normalize_playback_speed(settings.playback_speed);
    settings.time_display_mode = normalize_time_display_mode(&settings.time_display_mode);
    settings
}

fn sanitize_media_settings(mut settings: MediaPlaybackSettings) -> MediaPlaybackSettings {
    if let Some(id) = settings.subtitle_track_id
        && id <= 0
    {
        settings.subtitle_track_id = None;
    }
    settings
}

fn normalize_volume(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        DEFAULT_VOLUME
    }
}

fn normalize_loop_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "one" => "one".to_string(),
        "all" => "all".to_string(),
        _ => DEFAULT_LOOP_MODE.to_string(),
    }
}

fn normalize_hwdec_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "software" => "software".to_string(),
        _ => DEFAULT_HWDEC_MODE.to_string(),
    }
}

fn normalize_playback_speed(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED)
    } else {
        DEFAULT_PLAYBACK_SPEED
    }
}

fn normalize_time_display_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "frames" => "frames".to_string(),
        _ => DEFAULT_TIME_DISPLAY_MODE.to_string(),
    }
}

fn normalize_track_id(track_id: Option<i64>) -> Result<Option<i64>, String> {
    match track_id {
        Some(id) if id <= 0 => Err("invalid media playback track id".to_string()),
        other => Ok(other),
    }
}

fn normalize_network_stream_update(
    update: NetworkStreamHistoryUpdate,
) -> Result<NetworkStreamHistoryEntry, String> {
    let (url, scheme) = normalize_network_stream_url(&update.url)?;
    Ok(NetworkStreamHistoryEntry {
        name: update
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| network_stream_name_from_url(&url)),
        url,
        scheme,
        updated_at: update.updated_at.unwrap_or_else(now_millis).max(0),
    })
}

fn normalize_network_stream_url(url: &str) -> Result<(String, String), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err("network stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err("network stream url must include a protocol".to_string());
    };
    let scheme = scheme.to_ascii_lowercase();
    if !is_supported_network_stream_scheme(&scheme) {
        return Err(format!("unsupported network stream protocol: {scheme}"));
    }
    if rest.trim_matches('/').is_empty() {
        return Err("network stream url must include a host or path".to_string());
    }
    Ok((format!("{scheme}://{rest}"), scheme))
}

fn is_supported_network_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps"
    )
}

fn network_stream_key_for_url(url: &str) -> String {
    url.trim().to_string()
}

fn updated_index_key(updated_at: i64, path: &str) -> String {
    let newest_first = u64::MAX - updated_at.max(0) as u64;
    format!("{newest_first:020}|{path}")
}

fn store_key_for_path(path: &str) -> String {
    let trimmed = path.trim();
    let mut normalized = trimmed.replace('/', "\\");
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("\\\\?\\unc\\") {
        normalized = format!("\\\\{}", &normalized[8..]);
    } else if lower.starts_with("\\\\?\\") {
        normalized = normalized[4..].to_string();
    }

    if is_windows_drive_path(&normalized) || normalized.starts_with("\\\\") {
        normalized.to_lowercase()
    } else {
        trimmed.to_string()
    }
}

fn is_windows_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\'
}

fn get_by_normalized_or_legacy_key<'a, T>(
    table: &'a T,
    path: &str,
) -> Result<Option<redb::AccessGuard<'a, &'static str>>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let key = store_key_for_path(path);
    if let Some(stored) = table
        .get(key.as_str())
        .map_err(|error| format!("failed to read playback store entry: {error}"))?
    {
        return Ok(Some(stored));
    }

    let legacy_key = path.trim();
    if legacy_key == key {
        return Ok(None);
    }

    table
        .get(legacy_key)
        .map_err(|error| format!("failed to read playback store entry: {error}"))
}

fn media_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn network_stream_name_from_url(url: &str) -> String {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    if let Some(tail) = without_query
        .rsplit('/')
        .find(|part| !part.is_empty() && !part.contains("://"))
    {
        return tail.to_string();
    }
    let without_scheme = without_query
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(without_query);
    without_scheme
        .split('/')
        .next()
        .filter(|host| !host.is_empty())
        .unwrap_or(url)
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

    #[test]
    fn redb_store_matches_windows_history_paths_case_insensitively() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-history-windows-key-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&directory).expect("temp history directory should be created");
        let database_path = directory.join("history.redb");
        let mut store = PlaybackStore::open(database_path).expect("redb store should open");

        store
            .remember(PlaybackHistoryUpdate {
                path: "F:\\PP\\292MY-1051\\hhd800.com@292MY-1051.mp4".to_string(),
                name: None,
                position: 120.0,
                duration: 600.0,
                updated_at: Some(10),
            })
            .expect("entry should be written");

        let resume = store
            .resume_position("\\\\?\\f:\\pp\\292my-1051\\hhd800.com@292my-1051.mp4")
            .expect("resume lookup should normalize Windows paths");
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(resume, 120.0);
    }

    #[test]
    fn redb_store_persists_global_and_media_playback_settings() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-playback-settings-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&directory).expect("temp settings directory should be created");
        let database_path = directory.join("history.redb");
        let mut store = PlaybackStore::open(database_path).expect("redb store should open");

        store
            .update_settings(PlaybackSettingsUpdate {
                volume: Some(64.0),
                loop_mode: Some("all".to_string()),
                hwdec_mode: Some("software".to_string()),
                playback_speed: Some(1.25),
                video_fill: Some(true),
                time_display_mode: Some("frames".to_string()),
            })
            .expect("settings should be written");
        store
            .update_media_settings(
                "F:\\PP\\292MY-1051\\hhd800.com@292MY-1051.mp4",
                MediaPlaybackSettingsUpdate {
                    subtitle_track_id: Some(Some(3)),
                },
            )
            .expect("media settings should be written");

        drop(store);
        let store =
            PlaybackStore::open(directory.join("history.redb")).expect("redb store should reopen");
        let settings = store.settings().expect("settings should be readable");
        let media_settings = store
            .media_settings("\\\\?\\f:\\pp\\292my-1051\\hhd800.com@292my-1051.mp4")
            .expect("media settings should be readable");
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(settings.volume, 64.0);
        assert_eq!(settings.loop_mode, "all");
        assert_eq!(settings.hwdec_mode, "software");
        assert_eq!(settings.playback_speed, 1.25);
        assert!(settings.video_fill);
        assert_eq!(settings.time_display_mode, "frames");
        assert_eq!(media_settings.subtitle_track_id, Some(3));
    }

    #[test]
    fn redb_store_persists_network_stream_history_newest_first() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-network-streams-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&directory).expect("temp stream directory should be created");
        let database_path = directory.join("history.redb");
        let mut store = PlaybackStore::open(database_path).expect("redb store should open");

        store
            .remember_network_stream(NetworkStreamHistoryUpdate {
                url: "rtsp://camera.local/live".to_string(),
                name: None,
                updated_at: Some(10),
            })
            .expect("rtsp stream should be stored");
        store
            .remember_network_stream(NetworkStreamHistoryUpdate {
                url: "https://example.com/live/channel.m3u8".to_string(),
                name: Some("Example Live".to_string()),
                updated_at: Some(20),
            })
            .expect("https stream should be stored");
        store
            .remember_network_stream(NetworkStreamHistoryUpdate {
                url: "RTSP://camera.local/live".to_string(),
                name: Some("Front Door".to_string()),
                updated_at: Some(30),
            })
            .expect("same rtsp stream should update after protocol normalization");

        let entries = store
            .network_stream_history()
            .expect("network stream history should be readable");
        let _ = fs::remove_dir_all(&directory);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].url, "rtsp://camera.local/live");
        assert_eq!(entries[0].name, "Front Door");
        assert_eq!(entries[0].scheme, "rtsp");
        assert_eq!(entries[1].url, "https://example.com/live/channel.m3u8");
        assert_eq!(entries[1].name, "Example Live");
    }

    #[test]
    fn redb_store_clears_network_stream_history() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-network-streams-clear-{}-{}",
            std::process::id(),
            now_millis()
        ));
        fs::create_dir_all(&directory).expect("temp stream directory should be created");
        let database_path = directory.join("history.redb");
        let mut store = PlaybackStore::open(database_path).expect("redb store should open");

        store
            .remember_network_stream(NetworkStreamHistoryUpdate {
                url: "rtmp://example.com/live".to_string(),
                name: None,
                updated_at: Some(10),
            })
            .expect("rtmp stream should be stored");

        let entries = store
            .clear_network_stream_history()
            .expect("network stream history should clear");
        let after_clear = store
            .network_stream_history()
            .expect("network stream history should be readable");
        let _ = fs::remove_dir_all(&directory);

        assert!(entries.is_empty());
        assert!(after_clear.is_empty());
    }

    #[test]
    fn network_stream_history_rejects_unsupported_protocols() {
        let error = normalize_network_stream_update(NetworkStreamHistoryUpdate {
            url: "file:///C:/secret.mp4".to_string(),
            name: None,
            updated_at: Some(10),
        })
        .expect_err("local file urls should not be accepted as network streams");

        assert!(error.contains("unsupported network stream protocol"));
    }
}
