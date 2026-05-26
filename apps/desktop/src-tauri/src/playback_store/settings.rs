use redb::ReadableDatabase;

use super::{helpers::*, *};

impl PlaybackStore {
    pub(super) fn settings(&self) -> Result<PlaybackSettings, String> {
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

    pub(super) fn update_settings(
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

    pub(super) fn media_settings(&self, path: &str) -> Result<MediaPlaybackSettings, String> {
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

    pub(super) fn update_media_settings(
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
}
