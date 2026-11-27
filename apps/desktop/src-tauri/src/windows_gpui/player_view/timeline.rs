use std::time::{Duration, Instant};

use super::super::ui_state::UiState;

pub(super) const SAMPLE_INTERVAL: Duration = Duration::from_millis(160);
const CORRECTION_TIME: f64 = 0.24;
const MAX_CONTINUOUS_ERROR: f64 = 0.36;

pub(super) struct PlaybackClock {
    at: Instant,
    position: f64,
    rate: f64,
    correction: f64,
    generation: u64,
}

impl PlaybackClock {
    pub(super) fn new(state: &UiState) -> Self {
        Self {
            at: state.sampled_at.unwrap_or_else(Instant::now),
            position: state.position,
            rate: if state.advancing { state.speed } else { 0. },
            correction: 0.,
            generation: state.timeline_generation,
        }
    }

    pub(super) fn update(&mut self, state: &UiState) {
        let sample = Self::new(state);
        let projected = self.position_at(sample.at);
        let error = sample.position - projected;
        let continuous = self.generation == sample.generation
            && self.rate == sample.rate
            && sample.rate > 0.
            && error.abs() <= MAX_CONTINUOUS_ERROR;
        *self = if continuous {
            Self {
                position: projected,
                correction: error.max(-sample.rate * CORRECTION_TIME),
                ..sample
            }
        } else {
            sample
        };
    }

    fn position_at(&self, now: Instant) -> f64 {
        // An overdue sample may indicate buffering. Limit extrapolation until mpv responds.
        let elapsed = now
            .saturating_duration_since(self.at)
            .as_secs_f64()
            .min(0.45);
        self.position + self.rate * elapsed + self.correction * (elapsed / CORRECTION_TIME).min(1.)
    }

    pub(super) fn position(&self, now: Instant, duration: f64) -> f64 {
        self.position_at(now).clamp(0., duration.max(0.))
    }
}

pub(super) fn time_label(seconds: f64, state: &UiState) -> String {
    if state.time_display_frames
        && state.duration > 0.
        && let Some(fps) = state.fps.filter(|fps| fps.is_finite() && *fps > 0.)
    {
        let number = ((seconds.max(0.) * fps) as u64).to_string();
        let mut label = String::new();
        for (index, digit) in number.chars().enumerate() {
            if index > 0 && (number.len() - index).is_multiple_of(3) {
                label.push(',');
            }
            label.push(digit);
        }
        return label;
    }
    let seconds = seconds as u64;
    if state.duration > 3600. {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciles_small_decoder_sample_errors_without_reversing() {
        let start = Instant::now();
        let mut state = UiState {
            playing: true,
            advancing: true,
            speed: 1.,
            position: 4.,
            sampled_at: Some(start),
            ..Default::default()
        };
        let mut clock = PlaybackClock::new(&state);
        let now = start + SAMPLE_INTERVAL;
        let before = clock.position(now, 10.);
        state.position = 4.10;
        state.sampled_at = Some(now);
        clock.update(&state);
        assert_eq!(clock.position(now, 10.), before);
        assert!(clock.position(now + Duration::from_millis(16), 10.) > before);
    }

    #[test]
    fn seeks_and_pauses_follow_authoritative_samples_immediately() {
        let start = Instant::now();
        let mut state = UiState {
            playing: true,
            advancing: true,
            speed: 1.,
            position: 6.,
            sampled_at: Some(start),
            ..Default::default()
        };
        let mut clock = PlaybackClock::new(&state);
        state.position = 5.9;
        state.timeline_generation += 1;
        clock.update(&state);
        assert_eq!(clock.position(start, 10.), 5.9);
        state.playing = false;
        state.advancing = false;
        state.position = 6.;
        clock.update(&state);
        assert_eq!(clock.position(start + Duration::from_secs(5), 10.), 6.);
    }
}
