use std::sync::Mutex;

use openplayer_core::{CoreError, PlaybackService};
use openplayer_media::{
    MediaBackend, MediaError, MediaSource, MediaTime, PlaybackSnapshot, PlaybackStatus, Volume,
};
use serde::{Deserialize, Serialize};
use tauri::State;

pub struct DesktopPlaybackState {
    service: Mutex<PlaybackService<PreviewPlaybackBackend>>,
}

impl Default for DesktopPlaybackState {
    fn default() -> Self {
        Self {
            service: Mutex::new(PlaybackService::new(PreviewPlaybackBackend::default())),
        }
    }
}

impl DesktopPlaybackState {
    pub fn snapshot(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        let service = self.lock_service()?;
        Ok(PlaybackSnapshotDto::from(service.snapshot()))
    }

    pub fn open_preview_source(
        &self,
        source: PlaybackSourceDto,
    ) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        let source = source.try_into()?;
        self.with_service(|service| service.open(source))
    }

    pub fn play(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(PlaybackService::play)
    }

    pub fn pause(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(PlaybackService::pause)
    }

    pub fn stop(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(PlaybackService::stop)
    }

    pub fn seek(&self, position_ms: u64) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(|service| service.seek(MediaTime::from_millis(position_ms)))
    }

    pub fn set_volume(&self, percent: u16) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(|service| service.set_volume_percent(percent))
    }

    fn with_service(
        &self,
        command: impl FnOnce(
            &mut PlaybackService<PreviewPlaybackBackend>,
        ) -> Result<PlaybackSnapshot, CoreError>,
    ) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        let mut service = self.lock_service()?;
        command(&mut service)
            .map(|snapshot| PlaybackSnapshotDto::from(&snapshot))
            .map_err(PlaybackCommandError::from)
    }

    fn lock_service(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, PlaybackService<PreviewPlaybackBackend>>,
        PlaybackCommandError,
    > {
        self.service.lock().map_err(|_| {
            PlaybackCommandError::new("state.lockFailed", "Playback state is unavailable")
        })
    }
}

#[derive(Debug)]
struct PreviewPlaybackBackend {
    snapshot: PlaybackSnapshot,
}

impl Default for PreviewPlaybackBackend {
    fn default() -> Self {
        Self {
            snapshot: PlaybackSnapshot::idle(),
        }
    }
}

impl PreviewPlaybackBackend {
    fn require_source(&self) -> Result<(), MediaError> {
        if self.snapshot.source.is_some() {
            Ok(())
        } else {
            Err(MediaError::InvalidSource("no media opened".to_string()))
        }
    }
}

impl MediaBackend for PreviewPlaybackBackend {
    fn backend_id(&self) -> &'static str {
        "preview"
    }

    fn display_name(&self) -> &'static str {
        "HTML5 Preview Mirror"
    }

    fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
        self.snapshot.source = Some(source);
        self.snapshot.status = PlaybackStatus::Ready;
        self.snapshot.position = MediaTime::ZERO;
        self.snapshot.latest_error = None;
        Ok(self.snapshot.clone())
    }

    fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        self.require_source()?;
        self.snapshot.status = PlaybackStatus::Playing;
        Ok(self.snapshot.clone())
    }

    fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        self.require_source()?;
        self.snapshot.status = PlaybackStatus::Paused;
        Ok(self.snapshot.clone())
    }

    fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        self.require_source()?;
        self.snapshot.status = PlaybackStatus::Stopped;
        self.snapshot.position = MediaTime::ZERO;
        Ok(self.snapshot.clone())
    }

    fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
        self.snapshot.position = position;
        Ok(self.snapshot.clone())
    }

    fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
        self.snapshot.volume = volume;
        Ok(self.snapshot.clone())
    }

    fn snapshot(&self) -> PlaybackSnapshot {
        self.snapshot.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSourceDto {
    pub kind: PlaybackSourceKindDto,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackSourceKindDto {
    LocalFileLabel,
    LocalFolderLabel,
    HttpUrl,
}

impl TryFrom<PlaybackSourceDto> for MediaSource {
    type Error = PlaybackCommandError;

    fn try_from(source: PlaybackSourceDto) -> Result<Self, Self::Error> {
        match source.kind {
            PlaybackSourceKindDto::LocalFileLabel => Ok(MediaSource::local_file(source.value)),
            PlaybackSourceKindDto::LocalFolderLabel => Ok(MediaSource::local_folder(source.value)),
            PlaybackSourceKindDto::HttpUrl => MediaSource::http_url(source.value)
                .map_err(CoreError::from)
                .map_err(PlaybackCommandError::from),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshotDto {
    pub source_label: Option<String>,
    pub status: PlaybackStatusDto,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub volume_percent: u16,
    pub muted: bool,
    pub speed_milli: u32,
    pub latest_error: Option<PlaybackCommandError>,
}

impl From<&PlaybackSnapshot> for PlaybackSnapshotDto {
    fn from(snapshot: &PlaybackSnapshot) -> Self {
        Self {
            source_label: snapshot.source.as_ref().map(MediaSource::location),
            status: PlaybackStatusDto::from(snapshot.status),
            position_ms: snapshot.position.as_millis(),
            duration_ms: snapshot.duration.map(MediaTime::as_millis),
            volume_percent: snapshot.volume.percent(),
            muted: snapshot.muted,
            speed_milli: snapshot.speed.as_milli(),
            latest_error: snapshot
                .latest_error
                .clone()
                .map(CoreError::from)
                .map(PlaybackCommandError::from),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatusDto {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

impl From<PlaybackStatus> for PlaybackStatusDto {
    fn from(status: PlaybackStatus) -> Self {
        match status {
            PlaybackStatus::Idle => Self::Idle,
            PlaybackStatus::Loading => Self::Loading,
            PlaybackStatus::Ready => Self::Ready,
            PlaybackStatus::Playing => Self::Playing,
            PlaybackStatus::Paused => Self::Paused,
            PlaybackStatus::Stopped => Self::Stopped,
            PlaybackStatus::Ended => Self::Ended,
            PlaybackStatus::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCommandError {
    pub code: String,
    pub message: String,
}

impl PlaybackCommandError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<CoreError> for PlaybackCommandError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Media(media_error) => Self::from(media_error),
        }
    }
}

impl From<MediaError> for PlaybackCommandError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::BackendUnavailable(_) => Self::new(
                "media.backendUnavailable",
                "Playback backend is unavailable",
            ),
            MediaError::InvalidSource(_) => {
                Self::new("media.invalidSource", "This media source cannot be opened")
            }
            MediaError::OpenFailed(_) => {
                Self::new("media.openFailed", "The media source could not be opened")
            }
            MediaError::CommandFailed(_) => {
                Self::new("media.commandFailed", "Playback command failed")
            }
            MediaError::UnsupportedSource(_) => Self::new(
                "media.unsupportedSource",
                "This media source is not supported",
            ),
            MediaError::InvalidSeekTarget(_) => Self::new(
                "media.invalidSeekTarget",
                "The requested position is invalid",
            ),
            MediaError::InvalidVolume(_) => {
                Self::new("media.invalidVolume", "The requested volume is invalid")
            }
        }
    }
}

#[tauri::command]
pub fn playback_snapshot(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.snapshot()
}

#[tauri::command]
pub fn playback_open_preview_source(
    state: State<'_, DesktopPlaybackState>,
    source: PlaybackSourceDto,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.open_preview_source(source)
}

#[tauri::command]
pub fn playback_play(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.play()
}

#[tauri::command]
pub fn playback_pause(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.pause()
}

#[tauri::command]
pub fn playback_stop(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.stop()
}

#[tauri::command]
pub fn playback_seek(
    state: State<'_, DesktopPlaybackState>,
    position_ms: u64,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.seek(position_ms)
}

#[tauri::command]
pub fn playback_set_volume(
    state: State<'_, DesktopPlaybackState>,
    percent: u16,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.set_volume(percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_starts_idle() {
        let state = DesktopPlaybackState::default();

        let snapshot = state.snapshot().expect("snapshot");

        assert_eq!(snapshot.status, PlaybackStatusDto::Idle);
        assert_eq!(snapshot.source_label, None);
        assert_eq!(snapshot.position_ms, 0);
        assert_eq!(snapshot.volume_percent, 82);
    }

    #[test]
    fn opening_preview_file_sets_ready_snapshot() {
        let state = DesktopPlaybackState::default();

        let snapshot = state
            .open_preview_source(PlaybackSourceDto {
                kind: PlaybackSourceKindDto::LocalFileLabel,
                value: "movie.mp4".to_string(),
            })
            .expect("open source");

        assert_eq!(snapshot.status, PlaybackStatusDto::Ready);
        assert_eq!(snapshot.source_label, Some("movie.mp4".to_string()));
    }

    #[test]
    fn play_pause_stop_updates_snapshot_status() {
        let state = DesktopPlaybackState::default();
        state
            .open_preview_source(PlaybackSourceDto {
                kind: PlaybackSourceKindDto::LocalFileLabel,
                value: "movie.mp4".to_string(),
            })
            .expect("open source");

        assert_eq!(
            state.play().expect("play").status,
            PlaybackStatusDto::Playing
        );
        assert_eq!(
            state.pause().expect("pause").status,
            PlaybackStatusDto::Paused
        );
        let stopped = state.stop().expect("stop");
        assert_eq!(stopped.status, PlaybackStatusDto::Stopped);
        assert_eq!(stopped.position_ms, 0);
    }

    #[test]
    fn seek_and_volume_update_snapshot_values() {
        let state = DesktopPlaybackState::default();

        let seek_snapshot = state.seek(12_500).expect("seek");
        let volume_snapshot = state.set_volume(40).expect("volume");

        assert_eq!(seek_snapshot.position_ms, 12_500);
        assert_eq!(volume_snapshot.volume_percent, 40);
    }

    #[test]
    fn invalid_volume_returns_stable_error_code() {
        let state = DesktopPlaybackState::default();

        let error = state.set_volume(101).expect_err("invalid volume");

        assert_eq!(error.code, "media.invalidVolume");
    }

    #[test]
    fn play_before_open_returns_invalid_source() {
        let state = DesktopPlaybackState::default();

        let error = state.play().expect_err("no source");

        assert_eq!(error.code, "media.invalidSource");
    }
}
