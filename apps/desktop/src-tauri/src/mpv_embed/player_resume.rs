use super::*;

pub(super) fn initial_resume_seek_readiness(
    target_position: f64,
    duration: f64,
    seekable: bool,
) -> InitialResumeSeekReadiness {
    if !target_position.is_finite() || target_position <= 0.0 {
        return InitialResumeSeekReadiness::Skip;
    }

    if seekable || (duration.is_finite() && duration > 0.0 && target_position < duration) {
        return InitialResumeSeekReadiness::Ready;
    }

    if !duration.is_finite() || duration <= 0.0 || target_position >= duration {
        return InitialResumeSeekReadiness::Wait;
    }

    InitialResumeSeekReadiness::Wait
}

pub(super) fn is_transient_initial_resume_seek_error(error: &libmpv2::Error) -> bool {
    matches!(error, libmpv2::Error::Raw(code) if *code == libmpv2::mpv_error::Command)
}

impl MpvEmbedPlayer {
    pub(super) fn apply_initial_resume_seek(&mut self, resume_position: Option<f64>) {
        let Some(target_position) = normalize_initial_resume_position(resume_position) else {
            return;
        };

        let deadline = Instant::now() + INITIAL_RESUME_SEEK_TIMEOUT;

        loop {
            if !self.wait_for_initial_resume_seek(target_position, deadline) {
                return;
            }

            match self
                .mpv
                .command("seek", &[&target_position.to_string(), "absolute"])
            {
                Ok(()) => {
                    self.ended = false;
                    self.settle_initial_resume_seek(target_position);
                    return;
                }
                Err(error) if is_transient_initial_resume_seek_error(&error) => {
                    if Instant::now() >= deadline {
                        eprintln!("OpenPlayer initial resume seek timed out: {error}");
                        return;
                    }
                    self.wait_for_mpv_event(deadline, INITIAL_RESUME_SEEK_EVENT_WAIT);
                }
                Err(error) => {
                    eprintln!("OpenPlayer initial resume seek skipped: {error}");
                    return;
                }
            }
        }
    }

    pub(super) fn wait_for_initial_resume_seek(
        &mut self,
        target_position: f64,
        deadline: Instant,
    ) -> bool {
        loop {
            let duration = self.mpv.get_property::<f64>("duration").unwrap_or(0.0);
            let seekable = self.mpv.get_property::<bool>("seekable").unwrap_or(false);
            match initial_resume_seek_readiness(target_position, duration, seekable) {
                InitialResumeSeekReadiness::Ready => return true,
                InitialResumeSeekReadiness::Skip => return false,
                InitialResumeSeekReadiness::Wait => {}
            }

            let now = Instant::now();
            if now >= deadline {
                return false;
            }

            self.wait_for_mpv_event(deadline, INITIAL_RESUME_SEEK_EVENT_WAIT);
        }
    }

    pub(super) fn settle_initial_resume_seek(&mut self, target_position: f64) {
        let deadline = Instant::now() + INITIAL_RESUME_SEEK_SETTLE_TIMEOUT;

        loop {
            let position = self.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
            if position.is_finite()
                && (position - target_position).abs() <= INITIAL_RESUME_SEEK_TOLERANCE_SECONDS
            {
                return;
            }

            let now = Instant::now();
            if now >= deadline {
                return;
            }

            self.wait_for_mpv_event(deadline, INITIAL_RESUME_SEEK_EVENT_WAIT);
        }
    }
}
