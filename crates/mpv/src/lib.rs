use openplayer_media::{
    MediaBackend, MediaError, MediaSource, MediaTime, PlaybackSnapshot, Volume,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct MpvBackendDescriptor;

impl MpvBackendDescriptor {
    fn unavailable() -> MediaError {
        MediaError::BackendUnavailable("libmpv playback is not wired yet".to_string())
    }
}

impl MediaBackend for MpvBackendDescriptor {
    fn backend_id(&self) -> &'static str {
        "mpv"
    }

    fn display_name(&self) -> &'static str {
        "libmpv"
    }

    fn open(&mut self, _source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn seek(&mut self, _position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn set_volume(&mut self, _volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn snapshot(&self) -> PlaybackSnapshot {
        PlaybackSnapshot::idle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_media::{MediaBackendInfo, MediaError};

    #[test]
    fn exposes_mpv_backend_identity() {
        let descriptor = MpvBackendDescriptor;
        let info = MediaBackendInfo::from_backend(&descriptor);

        assert_eq!(info.backend_id, "mpv");
        assert_eq!(info.display_name, "libmpv");
    }

    #[test]
    fn playback_commands_report_unavailable_until_mpv_is_wired() {
        let expected =
            MediaError::BackendUnavailable("libmpv playback is not wired yet".to_string());
        let mut descriptor = MpvBackendDescriptor;

        assert_eq!(
            descriptor
                .open(MediaSource::local_file("movie.mp4"))
                .expect_err("open unavailable"),
            expected
        );
        assert_eq!(descriptor.play().expect_err("play unavailable"), expected);
        assert_eq!(descriptor.pause().expect_err("pause unavailable"), expected);
        assert_eq!(descriptor.stop().expect_err("stop unavailable"), expected);
        assert_eq!(
            descriptor
                .seek(MediaTime::from_millis(1))
                .expect_err("seek unavailable"),
            expected
        );
        assert_eq!(
            descriptor
                .set_volume(Volume::DEFAULT)
                .expect_err("volume unavailable"),
            expected
        );
    }
}
