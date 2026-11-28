use redb::{Database, TableDefinition};
use serde::{Deserialize, Serialize};

mod helpers;
mod history;
mod network_streams;
mod settings;
mod store;
#[cfg(test)]
mod tests;

pub(crate) fn network_stream(url: &str) -> Result<(String, String), String> {
    let (url, _) = helpers::normalize_network_stream_url(url)?;
    let name = helpers::network_stream_name_from_url(&url);
    Ok((url, name))
}

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
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) position: f64,
    pub(crate) duration: f64,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackHistoryUpdate {
    pub(crate) path: String,
    pub(crate) name: Option<String>,
    pub(crate) position: f64,
    pub(crate) duration: f64,
    pub(crate) updated_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettings {
    pub(crate) volume: f64,
    pub(crate) loop_mode: String,
    pub(crate) hwdec_mode: String,
    pub(crate) playback_speed: f64,
    pub(crate) video_fill: bool,
    pub(crate) time_display_mode: String,
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettingsUpdate {
    pub(crate) volume: Option<f64>,
    pub(crate) loop_mode: Option<String>,
    pub(crate) hwdec_mode: Option<String>,
    pub(crate) playback_speed: Option<f64>,
    pub(crate) video_fill: Option<bool>,
    pub(crate) time_display_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaybackSettings {
    path: String,
    pub(crate) subtitle_track_id: Option<i64>,
    pub(crate) has_subtitle_track_selection: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaybackSettingsUpdate {
    #[serde(default)]
    pub(crate) subtitle_track_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStreamHistoryEntry {
    pub(crate) url: String,
    pub(crate) name: String,
    scheme: String,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStreamHistoryUpdate {
    pub(crate) url: String,
    pub(crate) name: Option<String>,
    pub(crate) updated_at: Option<i64>,
}

pub(crate) struct PlaybackStore {
    database: Database,
}
