use std::time::Duration;

use gpui_ce_motion::{
    Animation, Easing, Frame, Keyframe, Motion, MotionState, Target, Timeline, Transition,
};

const SEARCH_HEIGHT: f32 = 53.;
const STAGGERED_CARDS: usize = 20;
const CARD_DELAY: Duration = Duration::from_millis(11);

pub(super) fn search_bar_frame(
    state: &mut MotionState,
    present: bool,
    now: Duration,
    reduced: bool,
) -> Frame {
    let hidden = Target::default().height(0.).opacity(0.);
    state.sample(
        Motion::new(Target::default().height(SEARCH_HEIGHT).opacity(1.))
            .initial(hidden)
            .exit(hidden)
            .present(present)
            .transition(Transition::Tween(
                Duration::from_millis(158),
                Easing::EaseOut,
            )),
        now,
        reduced,
    )
}

#[derive(Default)]
pub(super) struct ListingMotion {
    timeline: Option<Timeline>,
    restart: bool,
    running: bool,
}

impl ListingMotion {
    pub(super) fn restart(&mut self) {
        self.restart = true;
    }

    pub(super) fn sample(
        &mut self,
        now: Duration,
        reduced: bool,
        entry_count: usize,
    ) -> ListingFrame {
        if reduced {
            self.restart = false;
            self.running = false;
            return ListingFrame::default();
        }
        if !self.restart && !self.running {
            return ListingFrame::default();
        }
        let timeline = self.timeline.get_or_insert_with(listing_timeline);
        if self.restart {
            self.restart = false;
            timeline.restart(now);
        }
        let frames = timeline.sample(now, false);
        let listing = ListingFrame {
            offset: frames[0].target().y_px(),
            opacity: frames[0].target().alpha(),
            card_opacity: std::array::from_fn(|index| frames[index + 1].target().alpha()),
        };
        let last_animated = entry_count.min(STAGGERED_CARDS);
        self.running = frames[0].is_animating() || frames[last_animated].target().alpha() < 1.;
        listing
    }

    pub(super) fn is_animating(&self) -> bool {
        self.running
    }
}

#[derive(Clone, Copy)]
pub(super) struct ListingFrame {
    pub(super) offset: f32,
    pub(super) opacity: f32,
    card_opacity: [f32; STAGGERED_CARDS],
}

impl ListingFrame {
    pub(super) fn card_opacity(self, index: usize) -> f32 {
        self.card_opacity.get(index).copied().unwrap_or(1.)
    }
}

impl Default for ListingFrame {
    fn default() -> Self {
        Self {
            offset: 0.,
            opacity: 1.,
            card_opacity: [1.; STAGGERED_CARDS],
        }
    }
}

fn listing_timeline() -> Timeline {
    let mut timeline = Timeline::default();
    let entrance = |from, duration| {
        Animation::keyframes(
            duration,
            [
                Keyframe::new(0., from),
                Keyframe::new(1., Target::default().opacity(1.)).easing(Easing::EaseOut),
            ],
        )
        .expect("listing motion has fixed ordered keyframes")
    };
    timeline.at(
        Duration::ZERO,
        entrance(
            Target::default().y(5.).opacity(0.52),
            Duration::from_millis(135),
        ),
    );
    let card = entrance(Target::default().opacity(0.1), Duration::from_millis(122));
    for index in 0..STAGGERED_CARDS {
        timeline.at(CARD_DELAY * index as u32, card.clone());
    }
    timeline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_fades_in_order_and_stops_requesting_frames() {
        let mut listing = ListingMotion::default();
        listing.restart();
        let start = Duration::from_secs(2);
        listing.sample(start, false, STAGGERED_CARDS);
        let frame = listing.sample(start + Duration::from_millis(50), false, STAGGERED_CARDS);
        assert!(listing.is_animating());
        assert!(frame.opacity < 1.);
        assert!(frame.card_opacity(0) > frame.card_opacity(5));
        assert_eq!(frame.card_opacity(STAGGERED_CARDS), 1.);

        let completed = listing.sample(start + Duration::from_millis(400), false, STAGGERED_CARDS);
        assert_eq!(completed.opacity, 1.);
        assert_eq!(completed.card_opacity(STAGGERED_CARDS - 1), 1.);
        assert!(!listing.is_animating());

        listing.restart();
        assert_eq!(
            listing.sample(start, true, STAGGERED_CARDS).card_opacity(0),
            1.
        );
        assert!(!listing.is_animating());
    }
}
