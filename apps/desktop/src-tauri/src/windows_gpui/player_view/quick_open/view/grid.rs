use std::{ops::Range, time::Duration};

use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, Div, FontWeight, MouseButton, MouseDownEvent,
    ObjectFit, Pixels, Point, Stateful, StyledImage, UniformListDecoration, WeakEntity, Window,
    div, img, prelude::*, px, uniform_list,
};

use super::super::{DirectoryEntries, Entry, PlayerView, previews};
use super::{quick_tooltip, scrollbar};
use crate::windows_gpui::{theme::icon, widgets::button_feedback};

const CONTENT_PADDING: f32 = 16.;
const CARD_GAP: f32 = 12.;

impl PlayerView {
    pub(super) fn quick_contents(&self, width: f32, cx: &Context<Self>) -> Div {
        let picker = self.quick_open.as_ref().expect("browser is open");
        let root = div()
            .relative()
            .flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden();
        match picker.visible_entries() {
            DirectoryEntries::Ready(entries) if !entries.is_empty() => {
                let content_width = width - CONTENT_PADDING * 2.;
                let columns = (content_width / 178.).floor().clamp(3., 5.) as usize;
                let card_width = (content_width - CARD_GAP * (columns - 1) as f32) / columns as f32;
                let preview_height = (card_width * 0.57).clamp(68., 120.);
                root.child(
                    uniform_list(
                        "quick-grid",
                        entries.len().div_ceil(columns),
                        cx.processor(move |view, range, _, cx| {
                            view.quick_grid_rows(range, columns, preview_height, cx)
                        }),
                    )
                    .with_decoration(VisiblePreviews {
                        columns,
                        view: cx.weak_entity(),
                    })
                    .track_scroll(&picker.scroll)
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .pt(px(CONTENT_PADDING))
                    .px(px(CONTENT_PADDING))
                    .pb(px(CONTENT_PADDING - CARD_GAP)),
                )
                .child(scrollbar::quick_scrollbar(&picker.scroll, self.palette))
            }
            status => {
                let (glyph, key) = match status {
                    DirectoryEntries::Loading if picker.showing_search_results() => {
                        ("search", "media.quickOpen.searching")
                    }
                    DirectoryEntries::Unreadable if picker.showing_search_results() => {
                        ("info", "media.quickOpen.searchUnreadable")
                    }
                    DirectoryEntries::Ready(_) if picker.showing_search_results() => {
                        ("search", "media.quickOpen.searchEmpty")
                    }
                    DirectoryEntries::Loading => ("folder", "media.quickOpen.loading"),
                    DirectoryEntries::Unreadable => ("info", "media.quickOpen.unreadable"),
                    DirectoryEntries::Ready(_) if picker.show_media_favorites => {
                        ("bookmark", "media.quickOpen.emptyMediaFavorites")
                    }
                    DirectoryEntries::Ready(_) => ("folder", "media.quickOpen.empty"),
                };
                root.items_center().justify_center().child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_3()
                        .text_size(px(13.))
                        .text_color(self.palette.muted)
                        .child(icon(glyph, self.palette.faint, 34.))
                        .child(self.ui.label(key)),
                )
            }
        }
    }

    fn quick_grid_rows(
        &self,
        rows: Range<usize>,
        columns: usize,
        preview_height: f32,
        cx: &Context<Self>,
    ) -> Vec<Div> {
        let Some(picker) = &self.quick_open else {
            return Vec::new();
        };
        let DirectoryEntries::Ready(entries) = picker.visible_entries() else {
            return Vec::new();
        };
        let row_extra = if picker.showing_search_results() {
            17.
        } else {
            0.
        };
        rows.map(|row_index| {
            let cards = (row_index * columns..(row_index + 1) * columns).map(|index| {
                if let Some(entry) = entries.get(index) {
                    self.quick_card(
                        index,
                        entry,
                        picker.selected.contains(&index),
                        preview_height,
                        cx,
                    )
                    .into_any_element()
                } else {
                    div().flex_1().min_w_0().into_any_element()
                }
            });
            div()
                .flex()
                .w_full()
                .min_w_0()
                .h(px(preview_height + 64. + row_extra))
                .gap(px(CARD_GAP))
                .pb(px(CARD_GAP))
                .children(cards)
        })
        .collect()
    }

    fn quick_card(
        &self,
        index: usize,
        entry: &Entry,
        selected: bool,
        preview_height: f32,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let palette = self.palette;
        let location = self
            .quick_open
            .as_ref()
            .filter(|picker| picker.showing_search_results())
            .map(|picker| {
                entry
                    .path
                    .parent()
                    .and_then(|parent| parent.strip_prefix(&picker.directory).ok())
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .map_or_else(
                        || self.ui.label("media.quickOpen.currentFolder"),
                        |parent| parent.to_string_lossy().into_owned().into(),
                    )
            });
        let previewable = !entry.is_directory && previews::supports_preview(&entry.path);
        let show_bookmark = !entry.is_directory
            && self
                .quick_open
                .as_ref()
                .is_some_and(|picker| !picker.show_media_favorites)
            && self.quick_media_is_favorite(&entry.path);
        let context_path = entry.path.clone();
        let is_directory = entry.is_directory;
        let kind = if entry.is_directory {
            self.ui.label("media.quickOpen.folder")
        } else {
            entry
                .path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_uppercase()
                .into()
        };
        let glyph = if entry.is_directory {
            "folder"
        } else if previews::is_video_file(&entry.path) {
            "frames"
        } else {
            "headphones"
        };
        let thumbnail = previewable
            .then(|| self.quick_thumbnail(&entry.path))
            .flatten();
        let preview = div()
            .relative()
            .w_full()
            .h(px(preview_height))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .rounded(px(8.))
            .bg(if entry.is_directory {
                palette.accent_soft
            } else {
                palette.control
            })
            .child(if let Some(thumbnail) = thumbnail {
                img(thumbnail)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .rounded(px(8.))
                    .into_any_element()
            } else {
                icon(
                    glyph,
                    if entry.is_directory {
                        palette.accent
                    } else {
                        palette.faint
                    },
                    34.,
                )
                .into_any_element()
            })
            .when(selected, |el| {
                el.child(
                    div()
                        .absolute()
                        .top_2()
                        .right_2()
                        .size(px(20.))
                        .rounded(px(6.))
                        .bg(palette.accent)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon("check", palette.surface, 13.)),
                )
            });
        let card = div()
            .id(("quick-card", index))
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .rounded(px(11.))
            .border_1()
            .border_color(if selected {
                palette.accent_border
            } else {
                palette.line
            })
            .bg(if selected {
                palette.accent_soft
            } else {
                palette.control
            })
            .cursor_pointer()
            .child(preview)
            .child(
                div()
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(entry.name.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .min_w_0()
                            .text_size(px(11.))
                            .text_color(palette.muted)
                            .child(div().truncate().child(kind))
                            .when(show_bookmark, |el| {
                                el.child(icon("bookmark", palette.accent, 12.))
                            }),
                    )
                    .when_some(location.clone(), |el, location| {
                        el.child(
                            div()
                                .truncate()
                                .text_size(px(11.))
                                .text_color(palette.faint)
                                .child(location),
                        )
                    }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, _, cx| {
                    view.show_quick_entry_context(
                        context_path.clone(),
                        is_directory,
                        event.position,
                        cx,
                    );
                    cx.stop_propagation();
                }),
            )
            .on_click(cx.listener(move |view, event: &ClickEvent, _, cx| {
                view.click_quick_entry(index, event.click_count() > 1, cx);
            }));
        let card = button_feedback(card, palette);
        let card = if location.is_some() || entry.name.chars().count() > 20 {
            let details = if location.is_some() {
                entry.path.to_string_lossy().into_owned().into()
            } else {
                entry.name.clone()
            };
            quick_tooltip(card, details, palette).tooltip_show_delay(Duration::from_millis(1200))
        } else {
            card
        };
        if previewable {
            let path = entry.path.clone();
            card.on_hover(cx.listener(move |view, entered: &bool, _, cx| {
                view.quick_preview_hover(&path, *entered, cx);
            }))
        } else {
            card
        }
    }
}

struct VisiblePreviews {
    columns: usize,
    view: WeakEntity<PlayerView>,
}

impl UniformListDecoration for VisiblePreviews {
    fn compute(
        &self,
        visible: Range<usize>,
        _: Bounds<Pixels>,
        _: Point<Pixels>,
        _: Pixels,
        _: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let start = visible.start * self.columns;
        let end = visible.end * self.columns;
        let view = self.view.clone();
        window.defer(cx, move |_, cx| {
            let _ = view.update(cx, |view, cx| {
                let Some(picker) = &view.quick_open else {
                    return;
                };
                let DirectoryEntries::Ready(entries) = picker.visible_entries() else {
                    return;
                };
                let paths = entries[start.min(entries.len())..end.min(entries.len())]
                    .iter()
                    .filter(|entry| !entry.is_directory && previews::supports_preview(&entry.path))
                    .map(|entry| entry.path.clone())
                    .collect();
                view.queue_quick_thumbnails(paths, cx);
            });
        });
        div().into_any_element()
    }
}
