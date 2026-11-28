use std::{fs, path::PathBuf};

use super::{
    HISTORY_BY_PATH, HISTORY_BY_UPDATED, MEDIA_SETTINGS_BY_PATH, NETWORK_STREAMS_BY_UPDATED,
    NETWORK_STREAMS_BY_URL, PLAYBACK_SETTINGS, PlaybackStore, helpers::create_database_with_retry,
};

impl PlaybackStore {
    pub(crate) fn open(path: PathBuf) -> Result<Self, String> {
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
