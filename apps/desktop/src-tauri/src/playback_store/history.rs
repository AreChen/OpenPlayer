use redb::{ReadableDatabase, ReadableTable};

use super::{helpers::*, *};

impl PlaybackStore {
    pub(super) fn list(&self) -> Result<Vec<PlaybackHistoryEntry>, String> {
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

    pub(super) fn remember(
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

    pub(super) fn resume_position(&self, path: &str) -> Result<f64, String> {
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

    pub(super) fn clear(&mut self) -> Result<Vec<PlaybackHistoryEntry>, String> {
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
