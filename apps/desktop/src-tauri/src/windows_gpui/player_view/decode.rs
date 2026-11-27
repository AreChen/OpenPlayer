use std::time::Duration;

use gpui::{Context, Div, Stateful, Window, div, prelude::*, px};
use gpui_ce_motion::{Easing, Motion, Target, Transition};
use serde_json::Value;

use super::PlayerView;
use crate::windows_gpui::{
    theme::icon,
    widgets::{command, icon_button_box},
};

const GLYPH_SIZE: f32 = 20.;
const GLYPH_TRAVEL: f32 = 8.;

impl PlayerView {
    pub(in crate::windows_gpui) fn decode_button(
        &mut self,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let software_offset = if self.ui.software_decode {
            GLYPH_TRAVEL
        } else {
            0.
        };
        let switch = self.decode_motion.sample(
            Motion::new(Target::default().y(software_offset)).transition(Transition::Tween(
                Duration::from_millis(210),
                Easing::EaseInOut,
            )),
            self.clock.elapsed(),
            self.reduced_motion,
        );
        if switch.is_animating() {
            window.request_animation_frame();
        }

        let offset = switch.target().y_px();
        let progress = (offset / GLYPH_TRAVEL).clamp(0., 1.);
        let palette = self.palette;
        let glyph_layer = |name, top, alpha| {
            div()
                .absolute()
                .left_0()
                .top(px(top))
                .size(px(GLYPH_SIZE))
                .opacity(alpha)
                .child(icon(name, palette.accent, GLYPH_SIZE))
        };
        let glyph = div()
            .relative()
            .size(px(GLYPH_SIZE))
            .rounded(px(4.))
            .overflow_hidden()
            .when(progress < 1., |el| {
                el.child(glyph_layer("decodeHardware", -offset, 1. - progress))
            })
            .when(progress > 0., |el| {
                el.child(glyph_layer(
                    "decodeSoftware",
                    GLYPH_TRAVEL - offset,
                    progress,
                ))
            });

        command(
            icon_button_box("toggleDecode", palette, false).child(glyph),
            "toggleDecode",
            Value::Null,
            true,
            cx,
        )
    }
}
