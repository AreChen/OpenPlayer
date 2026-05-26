use tauri::State;

use super::*;
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
