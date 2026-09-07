use super::{NetworkStreamHistoryEntry, PlaybackHistoryEntry, PlaybackSettings, PlaybackStore};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSyncState {
    history: Vec<PlaybackHistoryEntry>,
    network_streams: Vec<NetworkStreamHistoryEntry>,
    settings: PlaybackSettings,
}

impl PlaybackStore {
    pub(super) fn sync_state(&self) -> Result<PlaybackSyncState, String> {
        Ok(PlaybackSyncState {
            history: self.list()?,
            network_streams: self.network_stream_history()?,
            settings: self.settings()?,
        })
    }
}
