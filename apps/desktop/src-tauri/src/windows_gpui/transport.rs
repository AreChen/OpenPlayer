use gpui::{Context, Div, FontWeight, Window, div, prelude::*, px};
use serde_json::Value;

use super::{
    player_view::PlayerView,
    ui_state::{MediaPanel, MenuPosition},
    widgets::{button_feedback, command, icon_button, slider, text_button, tooltip},
    window_layout::COMPACT_WIDTH,
};

impl PlayerView {
    pub(super) fn transport(
        &mut self,
        slide: f32,
        volume_allowed: bool,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Div {
        let compact = window.viewport_size().width < COMPACT_WIDTH;
        let volume = self.volume_button(volume_allowed, window, cx);
        let decode = (!compact).then(|| self.decode_button(window, cx));
        let (fullscreen_glyph, fullscreen_label) = if window.is_fullscreen() {
            ("exitFullscreen", "controls.exitFullscreen")
        } else {
            ("fullscreen", "contextMenu.fullscreen")
        };
        let ui = &self.ui;
        let palette = self.palette;
        let (position, current_time, total_time) = self.timeline();
        let label_width = (total_time.len() as f32 * 7.5).max(48.);
        let tip = |element, label| tooltip(element, ui.label(label), palette);
        let toggle_time = |id, value: &String| {
            command(
                tip(
                    button_feedback(
                        div()
                            .id(id)
                            .w(px(label_width))
                            .flex_none()
                            .rounded(px(5.))
                            .font_family("Consolas")
                            .text_size(px(12.5))
                            .text_color(palette.accent)
                            .child(value.clone()),
                        palette,
                    ),
                    if ui.time_display_frames {
                        "controls.showTime"
                    } else {
                        "controls.showFrames"
                    },
                ),
                "toggleTime",
                Value::Null,
                ui.loaded,
                cx,
            )
        };
        let timeline = div()
            .flex()
            .items_center()
            .gap(px(if compact { 4. } else { 8. }))
            .child(toggle_time("current-time", &current_time))
            .when(!compact, |el| {
                el.child(command(
                    tip(
                        icon_button("previous", "previous", palette, false)
                            .w(px(26.))
                            .h(px(22.)),
                        "controls.previousVideo",
                    ),
                    "previous",
                    Value::Null,
                    ui.queued_previous,
                    cx,
                ))
            })
            .child(tip(
                slider("seek", position, ui.duration, palette, cx),
                "controls.seek",
            ))
            .when(!compact, |el| {
                el.child(command(
                    tip(
                        icon_button("next", "next", palette, false)
                            .w(px(26.))
                            .h(px(22.)),
                        "controls.nextVideo",
                    ),
                    "next",
                    Value::Null,
                    ui.queued_next,
                    cx,
                ))
            })
            .child(toggle_time("duration", &total_time));
        let button = |kind, name, label, active, enabled| {
            command(
                tip(icon_button(kind, name, palette, active), label),
                kind,
                Value::Null,
                enabled,
                cx,
            )
        };
        let strip = div()
            .flex()
            .items_center()
            .gap(px(if compact { 4. } else { 8. }))
            .when(compact, |el| el.justify_between())
            .child(button(
                "openMedia",
                "openMedia",
                "controls.openMedia",
                false,
                !self.picker_open,
            ))
            .when(!compact, |el| {
                el.child(button(
                    "stopPlayback",
                    "stop",
                    "controls.stop",
                    false,
                    ui.loaded,
                ))
            })
            .child(button(
                "togglePlayback",
                if ui.playing { "pause" } else { "play" },
                if ui.playing {
                    "controls.pause"
                } else {
                    "controls.play"
                },
                true,
                !self.picker_open || ui.loaded,
            ))
            .when(!compact, |el| {
                el.child(button(
                    "toggleLoop",
                    "repeat",
                    "controls.openLoopMode",
                    ui.panel == Some(MediaPanel::Loop),
                    true,
                ))
            })
            .child(volume)
            .when_some(decode, |el, decode| {
                let speed_open = ui.panel == Some(MediaPanel::Speed);
                let speed = tip(
                    text_button("toggleSpeed", ui.speed_label.clone(), palette, speed_open)
                        .h(px(34.))
                        .min_w(px(36.))
                        .px(px(4.))
                        .when(!speed_open, |el| {
                            el.bg(gpui::transparent_black())
                                .border_color(gpui::transparent_black())
                        })
                        .text_color(palette.accent)
                        .font_weight(FontWeight::SEMIBOLD),
                    "controls.openPlaybackSpeed",
                );
                el.child(command(speed, "toggleSpeed", Value::Null, true, cx))
                    .child(decode)
                    .child(button(
                        "toggleTracks",
                        "subtitles",
                        "controls.openTracks",
                        ui.panel == Some(MediaPanel::Tracks),
                        true,
                    ))
                    .child(div().flex_1())
                    .child(button(
                        "togglePlaylist",
                        "list",
                        if ui.playlist {
                            "controls.hidePlaylist"
                        } else {
                            "controls.showPlaylist"
                        },
                        ui.playlist,
                        true,
                    ))
            })
            .child(tip(
                icon_button("fullscreen", fullscreen_glyph, palette, false)
                    .cursor_pointer()
                    .on_click(
                        cx.listener(|view, _, window, cx| view.toggle_fullscreen(window, cx)),
                    ),
                fullscreen_label,
            ))
            .when(compact, |el| {
                let menu_open = ui
                    .context
                    .as_ref()
                    .is_some_and(|menu| matches!(menu.position, MenuPosition::PlaybackControls));
                el.child(tip(
                    command(
                        icon_button("more-playback-controls", "more", palette, menu_open),
                        "compactContext",
                        Value::Null,
                        true,
                        cx,
                    ),
                    "controls.morePlaybackControls",
                ))
            });
        div()
            .absolute()
            .left(px(if compact { 10. } else { 24. }))
            .right(px(if compact { 10. } else { 24. }))
            .bottom(px(if compact { 10. } else { 24. }) - px(slide))
            .flex()
            .flex_col()
            .gap_2()
            .rounded(px(14.))
            .px(px(if compact { 8. } else { 14. }))
            .py(px(if compact { 8. } else { 12. }))
            .bg(palette.transport)
            .occlude()
            .child(timeline)
            .child(strip)
    }
}
