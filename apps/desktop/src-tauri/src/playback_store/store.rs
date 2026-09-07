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
        crate::store_schema::ensure_string_tables(
            &self.database,
            &[
                HISTORY_BY_PATH,
                HISTORY_BY_UPDATED,
                PLAYBACK_SETTINGS,
                MEDIA_SETTINGS_BY_PATH,
                NETWORK_STREAMS_BY_URL,
                NETWORK_STREAMS_BY_UPDATED,
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("failed to initialize store schema: {error}"))
    }
}
