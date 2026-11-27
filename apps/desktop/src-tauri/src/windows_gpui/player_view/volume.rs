use std::{cell::Cell, rc::Rc};

use gpui::{
    Bounds, Context, Div, FontWeight, MouseButton, MouseDownEvent, Pixels, Rgba, Window, canvas,
    div, prelude::*, px, relative,
};
use gpui_ce_motion::MotionState;
use serde_json::{Value, json};

use super::{PlayerView, motion};
use crate::windows_gpui::widgets::{command, icon_button};

const BUTTON_WIDTH: f32 = 36.;
const BUTTON_HEIGHT: f32 = 34.;
const POPOVER_WIDTH: f32 = 42.;
const POPOVER_HEIGHT: f32 = 142.;
const POPOVER_GAP: f32 = 6.;
const TRACK_HEIGHT: f32 = 89.;

#[derive(Default)]
pub(super) struct VolumeControl {
    hovered: bool,
    drag_track: Option<Bounds<Pixels>>,
    motion: MotionState,
}

impl VolumeControl {
    pub(super) fn reset(&mut self) {
        self.hovered = false;
        self.drag_track = None;
    }
}

impl PlayerView {
    pub(super) fn effective_volume(&self) -> f64 {
        if self.ui.muted { 0. } else { self.ui.volume }
    }

    pub(super) fn volume_text(&self) -> String {
        format!("{:.0}", self.effective_volume() * 100.)
    }

    pub(super) fn volume_glyph(&self) -> &'static str {
        match self.effective_volume() {
            value if value <= 0. => "volumeMuted",
            value if value <= 1. / 3. => "volumeLow",
            value if value <= 2. / 3. => "volumeMedium",
            _ => "volumeHigh",
        }
    }

    pub(in crate::windows_gpui) fn volume_button(
        &mut self,
        allowed: bool,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Div {
        let feedback = self
            .indicator
            .as_ref()
            .is_some_and(|(glyph, _)| glyph.starts_with("volume"));
        let revealed = allowed
            && (self.volume_control.hovered
                || self.volume_control.drag_track.is_some()
                || feedback);
        let animation = motion(
            &mut self.volume_control.motion,
            self.clock,
            revealed,
            self.reduced_motion,
            window,
        );
        let width = if revealed {
            POPOVER_WIDTH
        } else {
            BUTTON_WIDTH
        };
        let height = if revealed {
            BUTTON_HEIGHT + POPOVER_GAP + POPOVER_HEIGHT
        } else {
            BUTTON_HEIGHT
        };
        let button = command(
            icon_button("toggleMute", self.volume_glyph(), self.palette, revealed),
            "toggleMute",
            Value::Null,
            true,
            cx,
        );

        div()
            .relative()
            .flex_none()
            .w(px(BUTTON_WIDTH))
            .h(px(BUTTON_HEIGHT))
            .child(
                div()
                    .id("volume-hover")
                    .absolute()
                    .bottom_0()
                    .left(px((BUTTON_WIDTH - width) * 0.5))
                    .w(px(width))
                    .h(px(height))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .when(allowed, |el| {
                        el.on_hover(cx.listener(|view, hovered, _, cx| {
                            if view.volume_control.hovered != *hovered {
                                view.volume_control.hovered = *hovered;
                                cx.notify();
                            }
                        }))
                        .on_scroll(cx.listener(Self::wheel))
                    })
                    .child(button)
                    .when(animation.is_mounted(), |el| {
                        el.child(
                            self.volume_popover(cx)
                                .absolute()
                                .bottom(px(BUTTON_HEIGHT + POPOVER_GAP))
                                .left(px((width - POPOVER_WIDTH) * 0.5))
                                .opacity(animation.target().alpha().clamp(0., 1.)),
                        )
                    }),
            )
    }

    fn volume_popover(&self, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let volume = self.effective_volume();
        let ratio = volume as f32;
        let bounds = Rc::new(Cell::<Bounds<Pixels>>::default());
        let measured = bounds.clone();

        div()
            .w(px(POPOVER_WIDTH))
            .h(px(POPOVER_HEIGHT))
            .flex()
            .flex_col()
            .items_center()
            .justify_between()
            .pt(px(11.))
            .pb(px(9.))
            .rounded_full()
            .border_1()
            .border_color(palette.line)
            .bg(Rgba {
                a: 0.64 + palette.transport.a * 0.24,
                ..palette.transport
            })
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .w_full()
                    .h(px(16.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_center()
                    .font_family("Consolas")
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(palette.accent)
                    .child(self.volume_text()),
            )
            .child(
                div()
                    .id("vertical-volume-slider")
                    .w(px(34.))
                    .h(px(TRACK_HEIGHT + 14.))
                    .flex()
                    .justify_center()
                    .items_center()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, event: &MouseDownEvent, _, cx| {
                            let track = bounds.get();
                            view.drag_origin = None;
                            view.last_stage_click = None;
                            view.volume_control.drag_track = Some(track);
                            view.set_volume_at(event.position.y, track, cx);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(4.))
                            .h(px(TRACK_HEIGHT))
                            .rounded_full()
                            .bg(palette.line)
                            .child(
                                div()
                                    .absolute()
                                    .bottom_0()
                                    .w_full()
                                    .h(relative(ratio))
                                    .rounded_full()
                                    .bg(palette.accent),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .bottom(relative(ratio))
                                    .left(px(-3.))
                                    .mb(px(-5.))
                                    .size(px(10.))
                                    .rounded_full()
                                    .bg(palette.accent),
                            )
                            .child(
                                canvas(move |rect, _, _| measured.set(rect), |_, _, _, _| {})
                                    .absolute()
                                    .inset_0(),
                            ),
                    ),
            )
    }

    pub(super) fn move_volume_drag(
        &mut self,
        y: Pixels,
        pressed_button: Option<MouseButton>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(track) = self.volume_control.drag_track else {
            return false;
        };
        if pressed_button == Some(MouseButton::Left) {
            self.set_volume_at(y, track, cx);
            return true;
        }
        self.volume_control.drag_track = None;
        cx.notify();
        false
    }

    pub(super) fn finish_volume_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        if let Some(track) = self.volume_control.drag_track.take() {
            self.set_volume_at(y, track, cx);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn set_volume_at(&mut self, y: Pixels, track: Bounds<Pixels>, cx: &mut Context<Self>) {
        let height = f64::from(track.size.height);
        if height > 0. {
            let ratio = 1. - f64::from(y - track.origin.y) / height;
            let volume = (ratio.clamp(0., 1.) * 100.).round() / 100.;
            if self.ui.volume != volume || (self.ui.muted && volume > 0.) {
                self.send("volume", json!(volume), cx);
            }
        }
    }
}
