use super::*;

pub(super) fn startup_snapshot_for_interactive_open(
    path: &str,
    hwnd: i64,
    volume: f64,
    video_fill: bool,
    fallback_status: &str,
) -> MpvEmbedSnapshot {
    let paused = fallback_status == "paused";
    MpvEmbedSnapshot {
        path: path.to_string(),
        hwnd,
        status: fallback_status.to_string(),
        ended: false,
        paused,
        position: 0.0,
        duration: 0.0,
        fps: 0.0,
        speed: 1.0,
        hwdec: "auto-safe".to_string(),
        video_fill,
        subtitle_delay: 0.0,
        volume,
        tracks: Vec::new(),
    }
}

impl MpvEmbedPlayer {
    pub(super) fn recording_state(&self) -> MpvRecordingState {
        if let Some(recording) = &self.recording {
            MpvRecordingState {
                active: true,
                path: Some(recording.path.clone()),
                format: Some(recording.format.clone()),
            }
        } else {
            MpvRecordingState::inactive(None)
        }
    }

    pub(super) fn snapshot(&mut self, hwnd: i64, fallback_status: &str) -> MpvEmbedSnapshot {
        self.drain_events();
        if self.opening {
            return startup_snapshot_for_interactive_open(
                &self.path,
                hwnd,
                self.volume,
                self.video_fill,
                fallback_status,
            );
        }

        let raw_paused = self.mpv.get_property::<bool>("pause").unwrap_or(false);
        let pause_guard_active = self
            .force_paused_until
            .is_some_and(|deadline| Instant::now() < deadline);
        if !pause_guard_active {
            self.force_paused_until = None;
        }
        let paused = raw_paused || pause_guard_active;
        let ended = self.ended
            || self
                .mpv
                .get_property::<bool>("eof-reached")
                .unwrap_or(false);
        let position = self.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
        let duration = self.mpv.get_property::<f64>("duration").unwrap_or(0.0);
        let fps = read_player_fps(&self.mpv);
        let speed = self.mpv.get_property::<f64>("speed").unwrap_or(1.0);
        let hwdec = self
            .mpv
            .get_property::<String>("hwdec")
            .unwrap_or_else(|_| "auto-safe".to_string());
        let subtitle_delay = self.mpv.get_property::<f64>("sub-delay").unwrap_or(0.0);
        let tracks = read_tracks(&self.mpv);
        let percent_pos = self.mpv.get_property::<f64>("percent-pos").unwrap_or(0.0);
        let near_end = duration.is_finite()
            && duration > 0.0
            && position.is_finite()
            && duration - position <= END_OF_MEDIA_SNAP_TOLERANCE_SECONDS
            && percent_pos.is_finite()
            && percent_pos >= 99.0;

        MpvEmbedSnapshot {
            path: self.path.clone(),
            hwnd,
            status: if ended {
                "ended"
            } else if paused {
                "paused"
            } else {
                fallback_status
            }
            .to_string(),
            ended,
            paused,
            position: if (ended || near_end) && duration.is_finite() && duration > 0.0 {
                duration
            } else {
                position
            },
            duration,
            fps,
            speed,
            hwdec,
            video_fill: self.video_fill,
            subtitle_delay: if subtitle_delay.is_finite() {
                subtitle_delay
            } else {
                0.0
            },
            volume: self.volume,
            tracks,
        }
    }
}

impl MpvRecordingState {
    pub(super) fn inactive(path: Option<String>) -> Self {
        Self {
            active: false,
            path,
            format: None,
        }
    }
}
