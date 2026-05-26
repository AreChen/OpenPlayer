use std::{path::PathBuf, sync::Mutex};

use redb::{Database, TableDefinition};
use serde::{Deserialize, Serialize};

pub(crate) mod commands;
mod helpers;
mod history;
mod network_streams;
mod settings;
mod store;
#[cfg(test)]
mod tests;

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
    path: String,
    name: String,
    position: f64,
    duration: f64,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackHistoryUpdate {
    path: String,
    name: Option<String>,
    position: f64,
    duration: f64,
    updated_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettings {
    volume: f64,
    loop_mode: String,
    hwdec_mode: String,
    playback_speed: f64,
    video_fill: bool,
    time_display_mode: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSettingsUpdate {
    volume: Option<f64>,
    loop_mode: Option<String>,
    hwdec_mode: Option<String>,
    playback_speed: Option<f64>,
    video_fill: Option<bool>,
    time_display_mode: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaybackSettings {
    path: String,
    subtitle_track_id: Option<i64>,
    has_subtitle_track_selection: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPlaybackSettingsUpdate {
    #[serde(default)]
    subtitle_track_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStreamHistoryEntry {
    url: String,
    name: String,
    scheme: String,
    updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkStreamHistoryUpdate {
    url: String,
    name: Option<String>,
    updated_at: Option<i64>,
}

pub struct PlaybackStoreState {
    path: PathBuf,
    access: Mutex<()>,
}

struct PlaybackStore {
    database: Database,
}
