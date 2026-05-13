use openplayer_media::{
    MediaBackend, MediaBackendInfo, MediaError, MediaSource, MediaTime, PlaybackSnapshot, Volume,
};
use openplayer_shared::AppInfo;
use thiserror::Error;

pub fn app_info() -> AppInfo {
    AppInfo::skeleton(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error(transparent)]
    Media(#[from] MediaError),
}

pub struct PlaybackService<B: MediaBackend> {
    backend: B,
    snapshot: PlaybackSnapshot,
}

impl<B: MediaBackend> PlaybackService<B> {
    pub fn new(backend: B) -> Self {
        let snapshot = backend.snapshot();
        Self { backend, snapshot }
    }

    pub fn backend_info(&self) -> MediaBackendInfo {
        MediaBackendInfo::from_backend(&self.backend)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    pub fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, CoreError> {
        match &source {
            MediaSource::LocalFile(path) | MediaSource::LocalFolder(path) => {
                if path.as_os_str().is_empty() {
                    return Err(CoreError::Media(MediaError::InvalidSource(
                        source.location(),
                    )));
                }
            }
            MediaSource::HttpUrl(url) => {
                MediaSource::http_url(url.clone())?;
            }
        }

        self.apply_backend_result(|backend| backend.open(source))
    }

    pub fn play(&mut self) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(MediaBackend::play)
    }

    pub fn pause(&mut self) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(MediaBackend::pause)
    }

    pub fn stop(&mut self) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(MediaBackend::stop)
    }

    pub fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, CoreError> {
        if self
            .snapshot
            .duration
            .is_some_and(|duration| position.as_millis() > duration.as_millis())
        {
            return Err(CoreError::Media(MediaError::InvalidSeekTarget(
                position.as_millis(),
            )));
        }

        self.apply_backend_result(|backend| backend.seek(position))
    }

    pub fn set_volume_percent(&mut self, percent: u16) -> Result<PlaybackSnapshot, CoreError> {
        let volume = Volume::from_percent(percent)?;
        self.apply_backend_result(|backend| backend.set_volume(volume))
    }

    fn apply_backend_result(
        &mut self,
        command: impl FnOnce(&mut B) -> Result<PlaybackSnapshot, MediaError>,
    ) -> Result<PlaybackSnapshot, CoreError> {
        let snapshot = command(&mut self.backend)?;
        self.snapshot = snapshot.clone();
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_media::{
        MediaBackend, MediaError, MediaSource, MediaTime, PlaybackSnapshot, PlaybackStatus, Volume,
    };
    use openplayer_shared::AppStage;

    #[derive(Debug)]
    struct MockBackend {
        calls: Vec<&'static str>,
        snapshot: PlaybackSnapshot,
        next_error: Option<MediaError>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                snapshot: PlaybackSnapshot::idle(),
                next_error: None,
            }
        }

        fn fail_next(&mut self, error: MediaError) {
            self.next_error = Some(error);
        }

        fn maybe_fail(&mut self) -> Result<(), MediaError> {
            if let Some(error) = self.next_error.take() {
                Err(error)
            } else {
                Ok(())
            }
        }
    }

    impl MediaBackend for MockBackend {
        fn backend_id(&self) -> &'static str {
            "mock"
        }

        fn display_name(&self) -> &'static str {
            "Mock Backend"
        }

        fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("open");
            self.maybe_fail()?;
            self.snapshot.source = Some(source);
            self.snapshot.status = PlaybackStatus::Ready;
            Ok(self.snapshot.clone())
        }

        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("play");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Playing;
            Ok(self.snapshot.clone())
        }

        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("pause");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Paused;
            Ok(self.snapshot.clone())
        }

        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("stop");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Stopped;
            self.snapshot.position = MediaTime::ZERO;
            Ok(self.snapshot.clone())
        }

        fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("seek");
            self.maybe_fail()?;
            self.snapshot.position = position;
            Ok(self.snapshot.clone())
        }

        fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("set_volume");
            self.maybe_fail()?;
            self.snapshot.volume = volume;
            Ok(self.snapshot.clone())
        }

        fn snapshot(&self) -> PlaybackSnapshot {
            self.snapshot.clone()
        }
    }

    #[test]
    fn reports_openplayer_skeleton_info() {
        let info = app_info();

        assert_eq!(info.name, "OpenPlayer");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.stage, AppStage::Skeleton);
    }

    #[test]
    fn playback_service_opens_media_and_stores_ready_snapshot() {
        let source = MediaSource::local_file("movie.mp4");
        let mut service = PlaybackService::new(MockBackend::new());

        let snapshot = service.open(source.clone()).expect("open media");

        assert_eq!(snapshot.status, PlaybackStatus::Ready);
        assert_eq!(snapshot.source, Some(source));
        assert_eq!(service.snapshot(), &snapshot);
    }

    #[test]
    fn playback_service_rejects_empty_local_file_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service
            .open(MediaSource::local_file(""))
            .expect_err("empty source");

        assert_eq!(
            error,
            CoreError::Media(MediaError::InvalidSource(String::new()))
        );
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }

    #[test]
    fn playback_service_rejects_empty_http_url_target_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service
            .open(MediaSource::HttpUrl("https://".to_string()))
            .expect_err("empty url target");

        assert_eq!(
            error,
            CoreError::Media(MediaError::InvalidSource("https://".to_string()))
        );
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }

    #[test]
    fn playback_service_rejects_malformed_http_url_variant_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service
            .open(MediaSource::HttpUrl("http:///movie.mp4".to_string()))
            .expect_err("malformed target");

        assert_eq!(
            error,
            CoreError::Media(MediaError::InvalidSource("http:///movie.mp4".to_string()))
        );
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }

    #[test]
    fn playback_service_rejects_illegal_http_url_variant_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service
            .open(MediaSource::HttpUrl(
                "https://exa mple.test/movie.mp4".to_string(),
            ))
            .expect_err("illegal url characters");

        assert_eq!(
            error,
            CoreError::Media(MediaError::InvalidSource(
                "https://exa mple.test/movie.mp4".to_string()
            ))
        );
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }

    #[test]
    fn playback_service_rejects_unsupported_http_url_variant_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service
            .open(MediaSource::HttpUrl(
                "ftp://example.test/movie.mp4".to_string(),
            ))
            .expect_err("unsupported scheme");

        assert_eq!(
            error,
            CoreError::Media(MediaError::UnsupportedSource(
                "ftp://example.test/movie.mp4".to_string()
            ))
        );
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }

    #[test]
    fn playback_service_tracks_play_pause_stop_flow() {
        let mut service = PlaybackService::new(MockBackend::new());

        service
            .open(MediaSource::local_file("movie.mp4"))
            .expect("open");
        service.play().expect("play");
        assert_eq!(service.snapshot().status, PlaybackStatus::Playing);

        service.pause().expect("pause");
        assert_eq!(service.snapshot().status, PlaybackStatus::Paused);

        service.stop().expect("stop");
        assert_eq!(service.snapshot().status, PlaybackStatus::Stopped);
        assert_eq!(service.snapshot().position, MediaTime::ZERO);
    }

    #[test]
    fn playback_service_updates_seek_and_volume() {
        let mut service = PlaybackService::new(MockBackend::new());

        service.seek(MediaTime::from_millis(90_000)).expect("seek");
        service.set_volume_percent(40).expect("volume");

        assert_eq!(service.snapshot().position, MediaTime::from_millis(90_000));
        assert_eq!(
            service.snapshot().volume,
            Volume::from_percent(40).expect("volume")
        );
    }

    #[test]
    fn playback_service_rejects_seek_past_known_duration_before_backend_call() {
        let mut backend = MockBackend::new();
        backend.snapshot.duration = Some(MediaTime::from_millis(1_000));
        let mut service = PlaybackService::new(backend);

        let error = service
            .seek(MediaTime::from_millis(1_001))
            .expect_err("invalid seek");

        assert_eq!(
            error,
            CoreError::Media(MediaError::InvalidSeekTarget(1_001))
        );
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }

    #[test]
    fn playback_service_maps_backend_errors_to_core_errors() {
        let mut backend = MockBackend::new();
        backend.fail_next(MediaError::CommandFailed("play failed".to_string()));
        let mut service = PlaybackService::new(backend);

        let error = service.play().expect_err("backend error");

        assert_eq!(
            error,
            CoreError::Media(MediaError::CommandFailed("play failed".to_string()))
        );
    }

    #[test]
    fn playback_service_rejects_invalid_volume_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service.set_volume_percent(101).expect_err("invalid volume");

        assert_eq!(error, CoreError::Media(MediaError::InvalidVolume(101)));
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }
}
