use std::{fs, path::PathBuf, sync::Mutex};

use tauri::{AppHandle, Manager};

use super::{
    HISTORY_BY_PATH, HISTORY_BY_UPDATED, MEDIA_SETTINGS_BY_PATH, NETWORK_STREAMS_BY_UPDATED,
    NETWORK_STREAMS_BY_URL, PLAYBACK_SETTINGS, PlaybackStore, PlaybackStoreState,
    helpers::create_database_with_retry,
};

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

    pub(super) fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
        let mut directory = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
        directory.push("storage");
        Ok(directory.join("playback-history.redb"))
    }

    pub(super) fn with_store<T>(
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
    pub(super) fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create playback history directory: {error}"))?;
        }

        let database = create_database_with_retry(&path, "playback history")?;
        let store = Self { database };
        store.initialize()?;
        Ok(store)
    }

    pub(super) fn initialize(&self) -> Result<(), String> {
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
}
