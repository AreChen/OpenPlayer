use gpui::{Context, Div, FontWeight, Window, div, prelude::*, px};
use serde_json::Value;

use super::{
    player_view::PlayerView,
    ui_state::MediaPanel,
    widgets::{button_feedback, command, icon_button, slider, text_button},
};

impl PlayerView {
    pub(super) fn transport(
        &mut self,
        slide: f32,
        volume_allowed: bool,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Div {
        let volume = self.volume_button(volume_allowed, window, cx);
        let decode = self.decode_button(window, cx);
        let fullscreen_glyph = if window.is_fullscreen() {
            "exitFullscreen"
        } else {
            "fullscreen"
        };
        let ui = &self.ui;
        let palette = self.palette;
        let (position, current_time, total_time) = self.timeline();
        let label_width = (total_time.len() as f32 * 7.5).max(48.);
        let toggle_time = |id, value: &String| {
            command(
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
                "toggleTime",
                Value::Null,
                ui.loaded,
                cx,
            )
        };
        let timeline = div()
            .flex()
            .items_center()
            .gap_2()
            .child(toggle_time("current-time", &current_time))
            .child(command(
                icon_button("previous", "previous", palette, false)
                    .w(px(26.))
                    .h(px(22.)),
                "previous",
                Value::Null,
                ui.queued_previous,
                cx,
            ))
            .child(slider("seek", position, ui.duration, palette, cx))
            .child(command(
                icon_button("next", "next", palette, false)
                    .w(px(26.))
                    .h(px(22.)),
                "next",
                Value::Null,
                ui.queued_next,
                cx,
            ))
            .child(toggle_time("duration", &total_time));
        let button = |kind, name, active, enabled| {
            command(
                icon_button(kind, name, palette, active),
                kind,
                Value::Null,
                enabled,
                cx,
            )
        };
        let speed = text_button(
            "toggleSpeed",
            ui.speed_label.clone(),
            palette,
            ui.panel == Some(MediaPanel::Speed),
        )
        .h(px(34.))
        .min_w(px(54.))
        .bg(gpui::transparent_black())
        .border_color(gpui::transparent_black())
        .text_color(palette.accent)
        .font_weight(FontWeight::SEMIBOLD);
        let strip = div()
            .flex()
            .items_center()
            .gap_2()
            .child(button("openMedia", "openMedia", false, !self.picker_open))
            .child(button("stopPlayback", "stop", false, ui.loaded))
            .child(button(
                "togglePlayback",
                if ui.playing { "pause" } else { "play" },
                true,
                !self.picker_open || ui.loaded,
            ))
            .child(button(
                "toggleLoop",
                "repeat",
                ui.panel == Some(MediaPanel::Loop),
                true,
            ))
            .child(volume)
            .child(command(speed, "toggleSpeed", Value::Null, true, cx))
            .child(decode)
            .child(button(
                "toggleTracks",
                "subtitles",
                ui.panel == Some(MediaPanel::Tracks),
                true,
            ))
            .child(div().flex_1())
            .child(button("togglePlaylist", "list", ui.playlist, true))
            .child(
                icon_button("fullscreen", fullscreen_glyph, palette, false)
                    .cursor_pointer()
                    .on_click(
                        cx.listener(|view, _, window, cx| view.toggle_fullscreen(window, cx)),
                    ),
            );
        div()
            .absolute()
            .left(px(24.))
            .right(px(24.))
            .bottom(px(24. - slide))
            .flex()
            .flex_col()
            .gap_2()
            .rounded(px(14.))
            .px(px(14.))
            .py(px(12.))
            .bg(palette.transport)
            .occlude()
            .child(timeline)
            .child(strip)
    }
}
