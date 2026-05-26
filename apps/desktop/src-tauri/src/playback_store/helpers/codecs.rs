use super::{super::*, settings::*};

pub(in crate::playback_store) fn decode_entry(value: &str) -> Result<PlaybackHistoryEntry, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode playback history entry: {error}"))
}

pub(in crate::playback_store) fn decode_settings(value: &str) -> Result<PlaybackSettings, String> {
    serde_json::from_str(value)
        .map(sanitize_playback_settings)
        .map_err(|error| format!("failed to decode playback settings: {error}"))
}

pub(in crate::playback_store) fn decode_media_settings(
    value: &str,
) -> Result<MediaPlaybackSettings, String> {
    serde_json::from_str(value)
        .map(sanitize_media_settings)
        .map_err(|error| format!("failed to decode media playback settings: {error}"))
}

pub(in crate::playback_store) fn decode_network_stream_entry(
    value: &str,
) -> Result<NetworkStreamHistoryEntry, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode network stream history entry: {error}"))
}
