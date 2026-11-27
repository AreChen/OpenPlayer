use std::{
    fmt::Write as _,
    path::{Path, PathBuf},
};

use gpui::{
    ClipboardItem, Context, Div, FontWeight, MouseButton, Rgba, SharedString, Window, div,
    prelude::*, px,
};

use super::PlayerView;
use crate::windows_gpui::{
    media_info::{MediaInfo, MediaTrackInfo},
    theme::icon,
    widgets::{icon_button, text_button},
};

const PROPERTY_LABEL_WIDTH: f32 = 136.;
const COMPACT_PROPERTY_LABEL_WIDTH: f32 = 100.;
type PropertyRows = Vec<(&'static str, Option<String>)>;

pub(super) struct MediaProperties {
    pub(super) title: SharedString,
    pub(super) location: Option<PathBuf>,
    pub(super) status: MediaPropertiesStatus,
    pub(super) copied: bool,
}

pub(super) enum MediaPropertiesStatus {
    Loading,
    Ready(Box<MediaInfo>),
    Failed,
}

impl PlayerView {
    pub(super) fn media_properties_view(&self, window: &Window, cx: &Context<Self>) -> Div {
        let Some(properties) = &self.media_properties else {
            return div();
        };
        let palette = self.palette;
        let viewport = window.viewport_size();
        let close_label = self.ui.label("media.properties.close");
        let copy_label = self.ui.label(if properties.copied {
            "media.properties.copied"
        } else {
            "media.properties.copy"
        });
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .px_4()
            .h(px(52.))
            .bg(palette.control)
            .border_b_1()
            .border_color(palette.line)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(icon("info", palette.accent, 18.))
                    .child(self.ui.label("media.properties.title")),
            )
            .child(
                icon_button("media-properties-close", "close", palette, false)
                    .cursor_pointer()
                    .on_click(cx.listener(|view, _, _, cx| view.close_media_properties(cx))),
            );
        let footer = div()
            .flex()
            .flex_none()
            .justify_end()
            .gap_2()
            .px_4()
            .py_2()
            .bg(palette.control)
            .border_t_1()
            .border_color(palette.line)
            .when(
                !matches!(properties.status, MediaPropertiesStatus::Loading),
                |footer| {
                    footer.child(
                        text_button("media-properties-copy", copy_label, palette, false)
                            .min_w(px(100.))
                            .gap_2()
                            .flex_row_reverse()
                            .cursor_pointer()
                            .child(icon(
                                if properties.copied { "check" } else { "copy" },
                                palette.accent,
                                15.,
                            ))
                            .on_click(cx.listener(|view, _, _, cx| view.copy_media_properties(cx))),
                    )
                },
            )
            .child(
                text_button("media-properties-dismiss", close_label, palette, true)
                    .px_4()
                    .cursor_pointer()
                    .on_click(cx.listener(|view, _, _, cx| view.close_media_properties(cx))),
            );

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(Rgba {
                a: 0.62,
                ..palette.strong
            })
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| view.close_media_properties(cx)),
            )
            .child(
                div()
                    .w((viewport.width - px(48.)).min(px(600.)))
                    .h((viewport.height - px(40.)).min(px(620.)))
                    .occlude()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(17.))
                    .shadow_lg()
                    .bg(Rgba {
                        a: 1.,
                        ..palette.surface
                    })
                    .border_1()
                    .border_color(palette.line)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(header)
                    .child(
                        div()
                            .id("media-properties-scroll")
                            .min_h_0()
                            .flex_1()
                            .overflow_y_scroll()
                            .child(self.media_properties_content(properties)),
                    )
                    .child(footer),
            )
    }

    fn copy_media_properties(&mut self, cx: &mut Context<Self>) {
        let Some(properties) = &self.media_properties else {
            return;
        };
        let mut report = properties.title.to_string();
        match &properties.status {
            MediaPropertiesStatus::Loading => return,
            MediaPropertiesStatus::Failed => {
                if let Some(path) = &properties.location {
                    let label = self.ui.label("media.properties.location");
                    let _ = write!(report, "\n{label}: {}", path.display());
                }
                let _ = write!(report, "\n\n{}", self.ui.label("media.properties.failed"));
            }
            MediaPropertiesStatus::Ready(info) => {
                let mut has_properties = false;
                for (key, rows) in property_sections(info, properties.location.as_deref()) {
                    let mut values = rows
                        .into_iter()
                        .filter_map(|(label, value)| value.map(|value| (label, value)))
                        .peekable();
                    if values.peek().is_none() {
                        continue;
                    }
                    has_properties = true;
                    let _ = write!(report, "\n\n{}", self.ui.label(key));
                    for (label, value) in values {
                        let _ = write!(report, "\n{}: {value}", self.ui.label(label));
                    }
                }
                if !info.tracks.is_empty() {
                    let _ = write!(report, "\n\n{}", self.ui.label("media.properties.tracks"));
                    for track in &info.tracks {
                        let (label, details) = self.media_property_track_text(track);
                        let _ = write!(report, "\n{label}");
                        if !details.is_empty() {
                            let _ = write!(report, ": {details}");
                        }
                        if track.selected {
                            let _ =
                                write!(report, " ({})", self.ui.label("media.properties.selected"));
                        }
                    }
                } else if !has_properties {
                    let _ = write!(
                        report,
                        "\n\n{}",
                        self.ui.label("media.properties.noDetails")
                    );
                }
            }
        }
        cx.write_to_clipboard(ClipboardItem::new_string(report));
        if let Some(properties) = &mut self.media_properties {
            properties.copied = true;
        }
        cx.notify();
    }

    fn media_properties_content(&self, properties: &MediaProperties) -> Div {
        let mut content = div().flex().flex_col().gap_3().p_4().child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(19.))
                .child(properties.title.clone()),
        );

        match &properties.status {
            MediaPropertiesStatus::Ready(info) => {
                content = content
                    .children(self.media_property_sections(info, properties.location.as_deref()));
            }
            MediaPropertiesStatus::Loading | MediaPropertiesStatus::Failed => {
                if let Some(path) = &properties.location {
                    content = content.children(self.media_property_section(
                        "media.properties.general",
                        vec![(
                            "media.properties.location",
                            Some(path.display().to_string()),
                        )],
                        PROPERTY_LABEL_WIDTH,
                    ));
                }
                let loading = matches!(properties.status, MediaPropertiesStatus::Loading);
                content = content.child(self.media_property_status(if loading {
                    "media.properties.loading"
                } else {
                    "media.properties.failed"
                }));
            }
        }
        content
    }

    fn media_property_sections(&self, info: &MediaInfo, location: Option<&Path>) -> Vec<Div> {
        let mut sections = Vec::with_capacity(4);
        let [
            (general, general_rows),
            (video, video_rows),
            (audio, audio_rows),
        ] = property_sections(info, location);
        sections.extend(self.media_property_section(general, general_rows, PROPERTY_LABEL_WIDTH));
        let label_width = if info.video.is_some() && info.audio.is_some() {
            COMPACT_PROPERTY_LABEL_WIDTH
        } else {
            PROPERTY_LABEL_WIDTH
        };
        let video = self.media_property_section(video, video_rows, label_width);
        let audio = self.media_property_section(audio, audio_rows, label_width);
        if video.is_some() || audio.is_some() {
            sections.push(
                div().flex().items_start().gap_3().children(
                    video
                        .into_iter()
                        .chain(audio)
                        .map(|section| section.flex_1().min_w_0()),
                ),
            );
        }
        if !info.tracks.is_empty() {
            sections.push(
                self.media_property_group("media.properties.tracks")
                    .children(
                        info.tracks
                            .iter()
                            .map(|track| self.media_property_track(track)),
                    ),
            );
        }
        if sections.is_empty() {
            sections.push(self.media_property_status("media.properties.noDetails"));
        }
        sections
    }

    fn media_property_section(
        &self,
        label: &str,
        rows: PropertyRows,
        label_width: f32,
    ) -> Option<Div> {
        let rows = rows
            .into_iter()
            .filter_map(|(key, value)| value.map(|value| (key, value)));
        let mut rows = rows.peekable();
        rows.peek()?;
        Some(
            self.media_property_group(label)
                .children(rows.map(|(key, value)| {
                    div()
                        .flex()
                        .items_start()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .border_t_1()
                        .border_color(self.palette.line)
                        .child(
                            div()
                                .w(px(label_width))
                                .flex_none()
                                .text_color(self.palette.muted)
                                .child(self.ui.label(key)),
                        )
                        .child(div().flex_1().min_w_0().child(value))
                })),
        )
    }

    fn media_property_group(&self, label: &str) -> Div {
        div()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(11.))
            .border_1()
            .border_color(self.palette.line)
            .child(
                div()
                    .px_3()
                    .py_2()
                    .bg(self.palette.control)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.ui.label(label)),
            )
    }

    fn media_property_status(&self, label: &str) -> Div {
        div()
            .px_3()
            .py_4()
            .rounded(px(11.))
            .bg(self.palette.control)
            .text_color(self.palette.muted)
            .child(self.ui.label(label))
    }

    fn media_property_track(&self, track: &MediaTrackInfo) -> Div {
        let palette = self.palette;
        let (label, details) = self.media_property_track_text(track);
        div()
            .flex()
            .items_start()
            .gap_3()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(palette.line)
            .child(
                div()
                    .w(px(PROPERTY_LABEL_WIDTH))
                    .flex_none()
                    .text_color(palette.muted)
                    .child(label),
            )
            .child(div().flex_1().min_w_0().child(details))
            .when(track.selected, |row| {
                row.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .flex_none()
                        .text_color(palette.accent)
                        .child(icon("check", palette.accent, 13.))
                        .child(self.ui.label("media.properties.selected")),
                )
            })
    }

    fn media_property_track_text(&self, track: &MediaTrackInfo) -> (String, String) {
        let kind = match track.kind.as_str() {
            "video" => self.ui.label("media.properties.video"),
            "audio" => self.ui.label("media.properties.audio"),
            "sub" | "subtitle" => self.ui.label("media.properties.subtitles"),
            _ => SharedString::from(track.kind.clone()),
        };
        let name = self
            .ui
            .label("media.properties.track")
            .replace("{id}", &track.id.to_string());
        let details = [
            track.title.as_deref(),
            track.language.as_deref(),
            track.codec.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        (format!("{kind} · {name}"), details)
    }
}

fn property_sections(
    info: &MediaInfo,
    location: Option<&Path>,
) -> [(&'static str, PropertyRows); 3] {
    let general = vec![
        (
            "media.properties.location",
            location.map(|path| path.display().to_string()),
        ),
        (
            "media.properties.format",
            info.format
                .as_deref()
                .map(|format| format.replace(',', ", ")),
        ),
        ("media.properties.fileSize", info.size_bytes.map(file_size)),
        (
            "media.properties.duration",
            info.duration_seconds.map(duration),
        ),
        ("media.properties.bitrate", info.bitrate_bps.map(bitrate)),
    ];
    let video = info
        .video
        .as_ref()
        .map(|video| {
            vec![
                ("media.properties.codec", video.codec.clone()),
                ("media.properties.decoder", video.decoder.clone()),
                (
                    "media.properties.resolution",
                    video
                        .width
                        .zip(video.height)
                        .map(|(width, height)| format!("{width} × {height}")),
                ),
                ("media.properties.frameRate", video.fps.map(frame_rate)),
                ("media.properties.pixelFormat", video.format.clone()),
                ("media.properties.bitrate", video.bitrate_bps.map(bitrate)),
            ]
        })
        .unwrap_or_default();
    let audio = info
        .audio
        .as_ref()
        .map(|audio| {
            vec![
                ("media.properties.codec", audio.codec.clone()),
                ("media.properties.decoder", audio.decoder.clone()),
                (
                    "media.properties.channels",
                    audio.channels.map(|channels| channels.to_string()),
                ),
                (
                    "media.properties.sampleRate",
                    audio.sample_rate_hz.map(|rate| format!("{rate} Hz")),
                ),
                ("media.properties.sampleFormat", audio.format.clone()),
                ("media.properties.bitrate", audio.bitrate_bps.map(bitrate)),
            ]
        })
        .unwrap_or_default();
    [
        ("media.properties.general", general),
        ("media.properties.video", video),
        ("media.properties.audio", audio),
    ]
}

fn file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GiB", bytes as f64 / (1024. * 1024. * 1024.))
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MiB", bytes as f64 / (1024. * 1024.))
    } else if bytes >= 1024 {
        format!("{:.2} KiB", bytes as f64 / 1024.)
    } else {
        format!("{bytes} B")
    }
}

fn duration(seconds: f64) -> String {
    let total_ms = (seconds * 1000.).round() as u64;
    let total_seconds = total_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    let millis = total_ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn frame_rate(fps: f64) -> String {
    let exact = format!("{fps:.3}");
    format!("{} fps", exact.trim_end_matches('0').trim_end_matches('.'))
}

fn bitrate(bits_per_second: u64) -> String {
    format!("{:.1} kb/s", bits_per_second as f64 / 1000.)
}
