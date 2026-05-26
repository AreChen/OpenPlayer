use redb::{ReadableDatabase, ReadableTable};

use super::{helpers::*, *};

impl PlaybackStore {
    pub(super) fn network_stream_history(&self) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
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

    pub(super) fn remember_network_stream(
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

    pub(super) fn clear_network_stream_history(
        &mut self,
    ) -> Result<Vec<NetworkStreamHistoryEntry>, String> {
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
}
