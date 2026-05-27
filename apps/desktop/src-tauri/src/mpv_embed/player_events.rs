use super::*;

pub(super) fn handle_mpv_event(event: libmpv2::Result<Event<'_>>) -> MpvEventEffect {
    match event {
        Ok(Event::EndFile(mpv_end_file_reason::Eof)) => MpvEventEffect::Ended,
        Ok(Event::FileLoaded | Event::PlaybackRestart) => MpvEventEffect::Loaded,
        Ok(Event::StartFile | Event::Seek) => MpvEventEffect::Active,
        Ok(Event::LogMessage {
            prefix,
            level,
            text,
            ..
        }) => {
            log_mpv_video_diagnostic(prefix, level, text);
            MpvEventEffect::None
        }
        Err(error) => {
            eprintln!("OpenPlayer mpv event failed: {error}");
            MpvEventEffect::None
        }
        _ => MpvEventEffect::None,
    }
}

impl MpvEmbedPlayer {
    pub(super) fn wait_for_mpv_event(&mut self, deadline: Instant, max_wait: Duration) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }

        let wait = deadline
            .saturating_duration_since(now)
            .min(max_wait)
            .as_secs_f64();
        if let Some(event) = self.mpv.wait_event(wait) {
            let effect = handle_mpv_event(event);
            self.apply_mpv_event_effect(effect);
        }
    }

    pub(super) fn apply_mpv_event_effect(&mut self, effect: MpvEventEffect) {
        match effect {
            MpvEventEffect::Active => {
                self.ended = false;
            }
            MpvEventEffect::Loaded => {
                self.opening = false;
                self.ended = false;
            }
            MpvEventEffect::Ended => {
                self.opening = false;
                self.ended = true;
                let _ = stop_recording_for_player(self);
            }
            MpvEventEffect::None => {}
        }
    }

    pub(super) fn drain_events(&mut self) {
        while let Some(event) = self.mpv.wait_event(0.0) {
            let effect = handle_mpv_event(event);
            self.apply_mpv_event_effect(effect);
        }
    }
}
