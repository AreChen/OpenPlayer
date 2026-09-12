//! Drop-only submission pacing; never synthesize duplicate upstream frames.
pub(super) struct Cadence {
    period: Option<i128>,
    epoch: u64,
    next: Option<i128>,
    previous: Option<i128>,
}

impl Cadence {
    pub fn new(limit: Option<f64>) -> Self {
        Self {
            period: limit.map(|fps| (1e9 / fps).round() as i128),
            epoch: 0,
            next: None,
            previous: None,
        }
    }

    pub fn select(&mut self, epoch: u64, target: i64, fallback: u64) -> Option<u64> {
        if self.epoch != epoch {
            self.epoch = epoch;
            self.next = None;
            self.previous = None;
        }
        let fallback = fallback.max(self.period.unwrap_or(0) as u64);
        if target <= 0 {
            return Some(fallback);
        }
        let target = target as i128;
        if self.previous.is_some_and(|previous| target <= previous) {
            self.next = None;
            self.previous = None;
        }
        if let Some(period) = self.period {
            // Allow sub-millisecond clock rounding without drifting the phase.
            let rounded = target + (period / 100).min(100_000);
            if self.next.is_some_and(|next| rounded < next) {
                return None;
            }
            let base = self.next.unwrap_or(target);
            self.next = Some(base + ((rounded - base).max(0) / period + 1) * period);
        }
        let duration = self
            .previous
            .map(|previous| target - previous)
            .filter(|delta| (1..=10_000_000_000).contains(delta))
            .map(|delta| delta as u64)
            .unwrap_or(fallback);
        self.previous = Some(target);
        Some(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(source: f64, limit: f64, count: usize) -> Vec<u64> {
        let mut cadence = Cadence::new(Some(limit));
        (0..count)
            .filter_map(|i| cadence.select(1, 1_000_000_000 + (i as f64 * 1e9 / source) as i64, 1))
            .collect()
    }

    #[test]
    fn cap_never_upsamples_slow_upstream() {
        let frames = sample(15.0, 30.0, 150);
        assert_eq!(frames.len(), 150);
        assert!(
            frames[1..]
                .iter()
                .all(|duration| (66_666_666..=66_666_667).contains(duration))
        );
    }

    #[test]
    fn drops_without_phase_drift() {
        assert_eq!(sample(60.0, 30.0, 600).len(), 300);
        assert_eq!(sample(24.0, 15.0, 240).len(), 150);
    }

    #[test]
    fn uncapped_duration_uses_actual_timestamps() {
        let mut cadence = Cadence::new(None);
        assert_eq!(
            cadence.select(1, 1_000_000_000, 33_333_333),
            Some(33_333_333)
        );
        assert_eq!(
            cadence.select(1, 1_066_666_667, 33_333_333),
            Some(66_666_667)
        );
    }

    #[test]
    fn epoch_and_backwards_clock_reset_history() {
        let mut cadence = Cadence::new(Some(30.0));
        assert!(cadence.select(1, 1_000_000_000, 1).is_some());
        assert!(cadence.select(1, 1_010_000_000, 1).is_none());
        assert!(cadence.select(2, 1_010_000_000, 1).is_some());
        assert!(cadence.select(2, 1_000_000_000, 1).is_some());
        assert!(cadence.select(2, i64::MAX, 1).is_some());
        assert!(cadence.select(2, 0, 1).is_some());
    }
}
