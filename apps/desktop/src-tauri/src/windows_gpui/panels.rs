use gpui::{Context, Div, FontWeight, Pixels, SharedString, Window, div, prelude::*, px, rgba};
use serde_json::{Value, json};

use super::{
    player_view::{EditTarget, PlayerView},
    theme::icon,
    ui_state::{ContextMenu, MediaPanel, StreamDialog},
    widgets::{command, heading, icon_button, item_id, row, text_button, tooltip},
    window_layout::COMPACT_WIDTH,
};

fn panel_bottom(width: Pixels, regular: f32) -> Pixels {
    px(if width < px(436.) {
        136.
    } else if width < COMPACT_WIDTH {
        98.
    } else {
        regular
    })
}

impl PlayerView {
    pub(super) fn panels(
        &self,
        window: &Window,
        opacity: f32,
        slide: f32,
        cx: &Context<Self>,
    ) -> Div {
        let mut surface = div().absolute().inset_0().opacity(opacity);
        if self.ui.playlist {
            surface = surface.child(self.playlist(window, slide, cx));
        }
        if let Some(panel) = self.ui.panel {
            surface = surface.child(self.media_panel(panel, window, slide, cx));
        }
        if let Some(section) = self.ui.settings.as_deref() {
            surface = surface.child(self.settings_dialog(section, window, cx));
        }
        if let Some(stream) = self.ui.stream.as_ref() {
            surface = surface.child(self.stream_dialog(stream, window, cx));
        }
        surface
    }

    pub(super) fn context_menu(
        &self,
        menu: &ContextMenu,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let palette = self.palette;
        let viewport = window.viewport_size();
        let height = px((menu.items.len() as f32 * 30. + 18.).min(520.))
            .min((viewport.height - px(16.)).max(px(20.)));
        let x = px(menu.position.x)
            .max(px(8.))
            .min(viewport.width - px(258.));
        let y = px(menu.position.y)
            .max(px(8.))
            .min(viewport.height - height - px(8.));
        div()
            .absolute()
            .left(x)
            .top(y)
            .w(px(250.))
            .max_h(height)
            .rounded(px(12.))
            .border_1()
            .border_color(palette.line)
            .bg(palette.strong)
            .p_1()
            .occlude()
            .child(
                div()
                    .id("context-menu")
                    .max_h(height - px(8.))
                    .overflow_y_scroll()
                    .children(menu.items.iter().map(|item| {
                        if item.divider {
                            return div()
                                .h(px(1.))
                                .my_1()
                                .mx_2()
                                .bg(palette.line)
                                .into_any_element();
                        }
                        let mut entry = row(item_id("context-item", &item.id), palette, false)
                            .gap_2()
                            .min_h(px(29.))
                            .text_size(px(13.));
                        if let Some(glyph) = item.icon.as_ref() {
                            entry = entry.child(icon(glyph.clone(), palette.accent, 15.));
                        }
                        entry = entry.child(
                            div()
                                .flex_1()
                                .truncate()
                                .child(item.label.clone().unwrap_or_default()),
                        );
                        if let Some(shortcut) = item.shortcut.as_ref() {
                            entry = entry.child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(palette.faint)
                                    .child(shortcut.clone()),
                            );
                        }
                        let enabled = !item.disabled
                            && (item.id != "fitWindowToVideo"
                                || (!window.is_fullscreen() && !window.is_maximized()));
                        command(entry, "contextItem", json!(item.id), enabled, cx)
                            .into_any_element()
                    })),
            )
    }

    fn playlist(&self, window: &Window, slide: f32, cx: &Context<Self>) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        let width = window.viewport_size().width;
        let tip = |element, label| tooltip(element, ui.label(label), palette);
        let tab = |id, label, historical| {
            text_button(id, label, palette, historical == self.show_history)
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.show_history = historical;
                    cx.notify();
                }))
        };
        let controls = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(tab("queue-tab", ui.label("media.playlist"), false))
            .child(tab("recent-tab", ui.label("media.recent"), true))
            .child(div().flex_1())
            .child(command(
                tip(
                    icon_button("appendFiles", "folderAdd", palette, false).h(px(28.)),
                    "contextMenu.appendFiles",
                ),
                "appendFiles",
                Value::Null,
                !self.picker_open,
                cx,
            ))
            .child(command(
                tip(
                    icon_button("appendFolder", "folder", palette, false).h(px(28.)),
                    "contextMenu.appendFolder",
                ),
                "appendFolder",
                Value::Null,
                !self.picker_open,
                cx,
            ))
            .child(command(
                tip(
                    icon_button("hide-playlist", "close", palette, false).h(px(28.)),
                    "controls.hidePlaylist",
                ),
                "togglePlaylist",
                Value::Null,
                true,
                cx,
            ));
        let list = div()
            .id("native-playlist-items")
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_y_scroll();
        let list =
            if self.show_history {
                list.children(ui.history.iter().enumerate().map(|(index, entry)| {
                    command(
                        row(("history", index), palette, false)
                            .child(div().flex_1().truncate().child(entry.name.clone()))
                            .child(div().text_size(px(11.)).text_color(palette.faint).child(
                                format!("{} / {}", time(entry.position), time(entry.duration)),
                            )),
                        "historyItem",
                        json!(index),
                        true,
                        cx,
                    )
                }))
                .when(ui.history.is_empty(), |el| {
                    el.child(
                        div()
                            .p_3()
                            .text_color(palette.muted)
                            .child(ui.label("media.emptyPlaylist")),
                    )
                })
            } else {
                list.children(ui.queue.iter().enumerate().map(|(index, entry)| {
                    command(
                        row(
                            ("playlist-item", index),
                            palette,
                            ui.queue_index == Some(index),
                        )
                        .child(
                            div()
                                .text_color(palette.faint)
                                .min_w(px(20.))
                                .child(format!("{}", index + 1)),
                        )
                        .child(div().flex_1().truncate().child(entry.name.clone())),
                        "queueItem",
                        json!(index),
                        true,
                        cx,
                    )
                }))
                .when(ui.queue.is_empty(), |el| {
                    el.child(command(
                        row("add-to-empty-playlist", palette, false)
                            .child(ui.label("media.addFiles")),
                        "appendFiles",
                        Value::Null,
                        !self.picker_open,
                        cx,
                    ))
                })
            };
        div()
            .absolute()
            .right(if width < COMPACT_WIDTH {
                px(10.)
            } else {
                px(24.)
            })
            .top(px(52. + slide))
            .bottom(panel_bottom(width, 122.) - px(slide))
            .w(px(396.).min(width - px(20.)))
            .rounded(px(14.))
            .bg(palette.strong)
            .border_1()
            .border_color(palette.line)
            .occlude()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .child(controls)
            .child(list)
            .when(self.show_history && !ui.history.is_empty(), |el| {
                el.child(command(
                    text_button(
                        "clear-recent",
                        ui.label("settings.playback.clearHistory"),
                        palette,
                        false,
                    ),
                    "clearHistory",
                    Value::Null,
                    true,
                    cx,
                ))
            })
    }

    fn media_panel(
        &self,
        panel: MediaPanel,
        window: &Window,
        slide: f32,
        cx: &Context<Self>,
    ) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        let viewport = window.viewport_size();
        let bottom = panel_bottom(viewport.width, 117.) - px(slide);
        let (title, kind, width, body) =
            match panel {
                MediaPanel::Speed => {
                    let body = div().flex().flex_wrap().gap_2().children(
                        ui.speeds.iter().enumerate().map(|(index, entry)| {
                            command(
                                text_button(
                                    ("speed-option", index),
                                    entry.label.clone(),
                                    palette,
                                    (ui.speed - entry.value).abs() < 0.001,
                                ),
                                "speed",
                                json!(entry.value),
                                true,
                                cx,
                            )
                        }),
                    );
                    ("media.speed", "toggleSpeed", 328., body)
                }
                MediaPanel::Loop => {
                    let body = div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(ui.loops.iter().map(|entry| {
                            command(
                                row(
                                    item_id("loop-option", &entry.mode),
                                    palette,
                                    ui.loop_mode == entry.mode,
                                )
                                .items_start()
                                .flex_col()
                                .gap_0()
                                .py_2()
                                .child(entry.label.clone())
                                .child(
                                    div()
                                        .text_color(palette.faint)
                                        .text_size(px(12.))
                                        .child(entry.description.clone()),
                                ),
                                "loop",
                                json!(entry.mode),
                                true,
                                cx,
                            )
                        }));
                    ("media.loopMode", "toggleLoop", 304., body)
                }
                MediaPanel::Tracks => (
                    "contextMenu.tracksSubtitles",
                    "toggleTracks",
                    416.,
                    self.track_options(cx),
                ),
            };
        div()
            .absolute()
            .left(if viewport.width < COMPACT_WIDTH {
                px(10.)
            } else {
                px(24.)
            })
            .bottom(bottom)
            .w(px(width).min(viewport.width - px(20.)))
            .rounded(px(14.))
            .bg(palette.strong)
            .border_1()
            .border_color(palette.line)
            .occlude()
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .mb_2()
                    .child(heading(ui.label(title), palette))
                    .child(command(
                        tooltip(
                            icon_button("hide-media-panel", "close", palette, false).h(px(26.)),
                            ui.label("common.close"),
                            palette,
                        ),
                        kind,
                        Value::Null,
                        true,
                        cx,
                    )),
            )
            .child(
                div()
                    .id("media-options")
                    .max_h(px(306.).min(viewport.height - bottom - px(56.)))
                    .overflow_y_scroll()
                    .child(body),
            )
    }

    fn track_options(&self, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let ui = &self.ui;
        let layouts = [
            ("media.videoFit", "media.videoFitDescription", false),
            ("media.videoFill", "media.videoFillDescription", true),
        ];
        let mut content = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(heading(ui.label("media.videoLayout"), palette))
            .child(div().flex().gap_2().children(layouts.iter().map(
                |(label, description, fill)| {
                    command(
                        tooltip(
                            text_button(*label, ui.label(label), palette, ui.video_fill == *fill),
                            ui.label(description),
                            palette,
                        ),
                        "videoFill",
                        json!(fill),
                        true,
                        cx,
                    )
                },
            )));
        for (kind, label) in [
            ("audio", "media.audioTracks"),
            ("video", "media.videoTracks"),
            ("sub", "media.subtitles"),
        ] {
            let list = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(heading(ui.label(label), palette));
            let list = if kind == "sub" {
                list.child(command(
                    row("disable-subtitle", palette, false).child(ui.label("media.closeSubtitles")),
                    "track",
                    json!(["subtitle", null]),
                    ui.loaded,
                    cx,
                ))
            } else {
                list
            };
            content =
                content.child(
                    list.children(ui.tracks.iter().filter(|track| track.kind == kind).map(
                        |track| {
                            let name = track
                                .title
                                .as_ref()
                                .or(track.language.as_ref())
                                .or(track.codec.as_ref())
                                .cloned()
                                .unwrap_or_else(|| format!("#{}", track.id));
                            command(
                                row((kind, track.id as usize), palette, track.selected)
                                    .child(div().flex_1().truncate().child(name)),
                                "track",
                                json!([if kind == "sub" { "subtitle" } else { kind }, track.id]),
                                ui.loaded,
                                cx,
                            )
                        },
                    )),
                );
        }
        content
            .child(heading(ui.label("media.subtitleSync"), palette))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(command(
                        tooltip(
                            text_button("delay-minus", "−0.1 s", palette, false),
                            ui.label("media.subtitlesEarlier"),
                            palette,
                        ),
                        "subtitleDelay",
                        json!(ui.subtitle_delay - 0.1),
                        ui.loaded,
                        cx,
                    ))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(palette.muted)
                            .child(format!("{:+.1} s", ui.subtitle_delay)),
                    )
                    .child(command(
                        tooltip(
                            text_button("delay-plus", "+0.1 s", palette, false),
                            ui.label("media.subtitlesLater"),
                            palette,
                        ),
                        "subtitleDelay",
                        json!(ui.subtitle_delay + 0.1),
                        ui.loaded,
                        cx,
                    ))
                    .child(command(
                        tooltip(
                            text_button("delay-reset", ui.label("common.reset"), palette, false),
                            ui.label("media.resetSubtitleDelay"),
                            palette,
                        ),
                        "subtitleDelay",
                        json!(0),
                        ui.loaded,
                        cx,
                    )),
            )
            .child(command(
                text_button(
                    "addSubtitle",
                    ui.label("media.loadExternalSubtitle"),
                    palette,
                    false,
                ),
                "addSubtitle",
                Value::Null,
                ui.loaded && !self.picker_open,
                cx,
            ))
    }

    fn stream_dialog(&self, stream: &StreamDialog, window: &Window, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let ui = &self.ui;
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000076))
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(540.).min(window.viewport_size().width - px(24.)))
                    .max_h(px(450.))
                    .rounded(px(16.))
                    .bg(palette.surface)
                    .border_1()
                    .border_color(palette.line)
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .occlude()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(heading(ui.label("streamDialog.title"), palette))
                            .child(command(
                                tooltip(
                                    icon_button("closeNetworkStream", "close", palette, false),
                                    ui.label("common.close"),
                                    palette,
                                ),
                                "closeNetworkStream",
                                Value::Null,
                                true,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .text_color(palette.muted)
                            .child(ui.label("streamDialog.subtitle")),
                    )
                    .child(self.draft_field(
                        EditTarget::Stream,
                        ui.label("streamDialog.urlLabel"),
                        "",
                        cx,
                    ))
                    .when_some(stream.error.as_ref(), |el, error| {
                        el.child(div().text_color(palette.danger).child(error.clone()))
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(command(
                                text_button(
                                    "cancel-stream",
                                    ui.label("common.cancel"),
                                    palette,
                                    false,
                                ),
                                "closeNetworkStream",
                                Value::Null,
                                true,
                                cx,
                            ))
                            .child(
                                text_button(
                                    "submit-stream",
                                    ui.label("streamDialog.open"),
                                    palette,
                                    true,
                                )
                                .cursor_pointer()
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.commit_edit(cx);
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(heading(ui.label("streamDialog.recent"), palette))
                            .child(command(
                                text_button(
                                    "clear-network-history",
                                    ui.label("streamDialog.clearHistory"),
                                    palette,
                                    false,
                                ),
                                "clearNetworkHistory",
                                Value::Null,
                                !stream.history.is_empty(),
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .id("stream-history")
                            .max_h(px(120.))
                            .overflow_y_scroll()
                            .children(stream.history.iter().enumerate().map(|(index, item)| {
                                command(
                                    row(("stream-history-row", index), palette, false)
                                        .child(div().truncate().child(item.name.clone())),
                                    "streamHistoryItem",
                                    json!(index),
                                    true,
                                    cx,
                                )
                            }))
                            .when(stream.history.is_empty(), |el| {
                                el.child(
                                    div()
                                        .text_color(palette.faint)
                                        .child(ui.label("streamDialog.emptyHistory")),
                                )
                            }),
                    ),
            )
    }

    pub(super) fn draft_field(
        &self,
        target: EditTarget,
        placeholder: SharedString,
        initial: &str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let palette = self.palette;
        let active = self.edit.as_ref().filter(|(edit, _)| *edit == target);
        let shown = active.map_or(initial, |(_, value)| value.as_str());
        let label = if shown.is_empty() {
            placeholder
        } else {
            shown.to_owned().into()
        };
        let active = active.is_some();
        let text = initial.to_owned();
        let id = match &target {
            EditTarget::Stream => "stream-url".to_owned(),
            EditTarget::Accent => "accent-text".to_owned(),
        };
        div()
            .id(SharedString::from(id))
            .min_w(px(120.))
            .flex_1()
            .min_h(px(30.))
            .px_3()
            .py_1()
            .bg(palette.control)
            .border_1()
            .border_color(if active { palette.accent } else { palette.line })
            .rounded_md()
            .hover(move |style| style.border_color(palette.accent))
            .cursor_text()
            .font_weight(FontWeight::NORMAL)
            .text_color(if shown.is_empty() {
                palette.faint
            } else {
                palette.text
            })
            .child(div().truncate().child(label))
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.edit.as_ref().is_none_or(|(edit, _)| *edit != target) {
                    this.edit = Some((target.clone(), text.clone()));
                    cx.notify();
                }
            }))
    }
}

fn time(seconds: f64) -> String {
    let secs = seconds.max(0.) as u64;
    format!("{}:{:02}", secs / 60, secs % 60)
}
