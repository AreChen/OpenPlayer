use std::path::PathBuf;

use thiserror::Error;
use url::Url;

pub trait MediaBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError>;
    fn play(&mut self) -> Result<PlaybackSnapshot, MediaError>;
    fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError>;
    fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError>;
    fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError>;
    fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError>;
    fn snapshot(&self) -> PlaybackSnapshot;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaBackendInfo {
    pub backend_id: String,
    pub display_name: String,
}

impl MediaBackendInfo {
    pub fn from_backend(backend: &dyn MediaBackend) -> Self {
        Self {
            backend_id: backend.backend_id().to_string(),
            display_name: backend.display_name().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaSource {
    LocalFile(PathBuf),
    LocalFolder(PathBuf),
    HttpUrl(String),
}

impl MediaSource {
    pub fn local_file(path: impl Into<PathBuf>) -> Self {
        Self::LocalFile(path.into())
    }

    pub fn local_folder(path: impl Into<PathBuf>) -> Self {
        Self::LocalFolder(path.into())
    }

    pub fn http_url(url: impl Into<String>) -> Result<Self, MediaError> {
        let url = url.into();

        let parsed = match Url::parse(&url) {
            Ok(parsed) => parsed,
            Err(_)
                if url
                    .get(..7)
                    .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
                    || url
                        .get(..8)
                        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://")) =>
            {
                return Err(MediaError::InvalidSource(url));
            }
            Err(_) => return Err(MediaError::UnsupportedSource(url)),
        };

        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(MediaError::UnsupportedSource(url));
        }

        if url
            .split_once("://")
            .is_some_and(|(_, target)| target.starts_with('/'))
        {
            return Err(MediaError::InvalidSource(url));
        }

        if parsed.host().is_none() {
            return Err(MediaError::InvalidSource(url));
        }

        Ok(Self::HttpUrl(url))
    }

    pub fn location(&self) -> String {
        match self {
            Self::LocalFile(path) | Self::LocalFolder(path) => path.to_string_lossy().into_owned(),
            Self::HttpUrl(url) => url.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaTime(u64);

impl MediaTime {
    pub const ZERO: Self = Self(0);

    pub const fn from_millis(milliseconds: u64) -> Self {
        Self(milliseconds)
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume(u16);

impl Volume {
    pub const DEFAULT: Self = Self(82);

    pub const fn percent(self) -> u16 {
        self.0
    }

    pub fn from_percent(percent: u16) -> Result<Self, MediaError> {
        if percent <= 100 {
            Ok(Self(percent))
        } else {
            Err(MediaError::InvalidVolume(percent))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackSpeed(u32);

impl PlaybackSpeed {
    pub const NORMAL: Self = Self(1000);

    pub const fn from_milli(milli: u32) -> Self {
        Self(milli)
    }

    pub const fn as_milli(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackSnapshot {
    pub source: Option<MediaSource>,
    pub status: PlaybackStatus,
    pub position: MediaTime,
    pub duration: Option<MediaTime>,
    pub volume: Volume,
    pub muted: bool,
    pub speed: PlaybackSpeed,
    pub latest_error: Option<MediaError>,
}

impl PlaybackSnapshot {
    pub const fn idle() -> Self {
        Self {
            source: None,
            status: PlaybackStatus::Idle,
            position: MediaTime::ZERO,
            duration: None,
            volume: Volume::DEFAULT,
            muted: false,
            speed: PlaybackSpeed::NORMAL,
            latest_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackEvent {
    StateChanged(PlaybackSnapshot),
    PositionChanged(MediaTime),
    MediaOpened(MediaSource),
    MediaEnded,
    BackendError(MediaError),
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MediaError {
    #[error("media backend is unavailable: {0}")]
    BackendUnavailable(String),
    #[error("invalid media source: {0}")]
    InvalidSource(String),
    #[error("failed to open media: {0}")]
    OpenFailed(String),
    #[error("media command failed: {0}")]
    CommandFailed(String),
    #[error("unsupported media source: {0}")]
    UnsupportedSource(String),
    #[error("invalid seek target: {0}")]
    InvalidSeekTarget(u64),
    #[error("invalid volume: {0}")]
    InvalidVolume(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBackend;

    struct RecordingBackend {
        calls: Vec<&'static str>,
        snapshot: PlaybackSnapshot,
    }

    impl MediaBackend for TestBackend {
        fn backend_id(&self) -> &'static str {
            "test"
        }

        fn display_name(&self) -> &'static str {
            "Test Backend"
        }

        fn open(&mut self, _source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn seek(&mut self, _position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn set_volume(&mut self, _volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn snapshot(&self) -> PlaybackSnapshot {
            PlaybackSnapshot::idle()
        }
    }

    impl MediaBackend for RecordingBackend {
        fn backend_id(&self) -> &'static str {
            "recording"
        }

        fn display_name(&self) -> &'static str {
            "Recording Backend"
        }

        fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("open");
            self.snapshot.source = Some(source);
            self.snapshot.status = PlaybackStatus::Ready;
            Ok(self.snapshot.clone())
        }

        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("play");
            self.snapshot.status = PlaybackStatus::Playing;
            Ok(self.snapshot.clone())
        }

        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("pause");
            self.snapshot.status = PlaybackStatus::Paused;
            Ok(self.snapshot.clone())
        }

        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("stop");
            self.snapshot.status = PlaybackStatus::Stopped;
            self.snapshot.position = MediaTime::ZERO;
            Ok(self.snapshot.clone())
        }

        fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("seek");
            self.snapshot.position = position;
            Ok(self.snapshot.clone())
        }

        fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("set_volume");
            self.snapshot.volume = volume;
            Ok(self.snapshot.clone())
        }

        fn snapshot(&self) -> PlaybackSnapshot {
            self.snapshot.clone()
        }
    }

    #[test]
    fn backend_info_is_derived_from_trait() {
        let info = MediaBackendInfo::from_backend(&TestBackend);

        assert_eq!(info.backend_id, "test");
        assert_eq!(info.display_name, "Test Backend");
    }

    #[test]
    fn backend_commands_update_snapshot_and_record_call_order() {
        let source = MediaSource::local_file("movie.mp4");
        let mut backend = RecordingBackend {
            calls: Vec::new(),
            snapshot: PlaybackSnapshot::idle(),
        };

        backend.open(source.clone()).expect("open media");
        backend.play().expect("play media");
        backend
            .seek(MediaTime::from_millis(42_000))
            .expect("seek media");
        backend
            .set_volume(Volume::from_percent(55).expect("valid volume"))
            .expect("set volume");
        backend.pause().expect("pause media");
        let snapshot = backend.stop().expect("stop media");

        assert_eq!(
            backend.calls,
            ["open", "play", "seek", "set_volume", "pause", "stop"]
        );
        assert_eq!(snapshot.source, Some(source));
        assert_eq!(snapshot.status, PlaybackStatus::Stopped);
        assert_eq!(snapshot.position, MediaTime::ZERO);
        assert_eq!(
            snapshot.volume,
            Volume::from_percent(55).expect("valid volume")
        );
    }

    #[test]
    fn media_source_accepts_http_and_https_urls() {
        let http = MediaSource::http_url("http://example.test/movie.mp4").expect("http source");
        let https = MediaSource::http_url("https://example.test/movie.mp4").expect("https source");

        assert_eq!(http.location(), "http://example.test/movie.mp4");
        assert_eq!(https.location(), "https://example.test/movie.mp4");
    }

    #[test]
    fn media_source_rejects_unsupported_urls() {
        let error =
            MediaSource::http_url("ftp://example.test/movie.mp4").expect_err("unsupported scheme");

        assert_eq!(
            error,
            MediaError::UnsupportedSource("ftp://example.test/movie.mp4".to_string())
        );
    }

    #[test]
    fn media_source_rejects_empty_http_url_target() {
        let error = MediaSource::http_url("https://").expect_err("empty url target");

        assert_eq!(error, MediaError::InvalidSource("https://".to_string()));
    }

    #[test]
    fn media_source_rejects_malformed_http_targets() {
        for value in [
            "https:// ",
            "http:///movie.mp4",
            "https://?x",
            "https://#fragment",
        ] {
            let error = MediaSource::http_url(value).expect_err("malformed url target");
            assert_eq!(error, MediaError::InvalidSource(value.to_string()));
        }
    }

    #[test]
    fn media_source_rejects_http_urls_with_illegal_characters() {
        let value = "https://exa mple.test/movie.mp4";
        let error = MediaSource::http_url(value).expect_err("illegal url characters");

        assert_eq!(error, MediaError::InvalidSource(value.to_string()));
    }

    #[test]
    fn media_source_treats_malformed_uppercase_http_urls_as_invalid() {
        let value = "HTTPS://exa mple.test/movie.mp4";
        let error = MediaSource::http_url(value).expect_err("malformed uppercase http url");

        assert_eq!(error, MediaError::InvalidSource(value.to_string()));
    }

    #[test]
    fn playback_snapshot_starts_idle_with_defaults() {
        let snapshot = PlaybackSnapshot::idle();

        assert_eq!(snapshot.status, PlaybackStatus::Idle);
        assert_eq!(snapshot.position, MediaTime::ZERO);
        assert_eq!(snapshot.duration, None);
        assert_eq!(snapshot.volume, Volume::DEFAULT);
        assert_eq!(snapshot.speed, PlaybackSpeed::NORMAL);
        assert!(!snapshot.muted);
    }

    #[test]
    fn volume_rejects_values_above_100_percent() {
        let error = Volume::from_percent(101).expect_err("invalid volume");

        assert_eq!(error, MediaError::InvalidVolume(101));
    }
}
