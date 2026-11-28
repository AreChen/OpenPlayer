mod entry_menu;
mod grid;
mod navigation;
mod scrollbar;
mod toolbar;

use gpui::{Context, Div, FontWeight, MouseButton, Rgba, Window, div, prelude::*, px};

use super::{
    DirectoryEntries, PlayerView, QuickOpenMode, address::input_field, motion::search_bar_frame,
    search::QuickSearch,
};
use crate::player::{
    theme::icon,
    widgets::{icon_button, text_button, tooltip as quick_tooltip},
    window_layout::COMPACT_WIDTH,
};

pub(super) use entry_menu::QuickEntryContext;
pub(super) use toolbar::ToolbarMenu;

impl PlayerView {
    pub(in crate::player::player_view) fn quick_open_view(
        &mut self,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Div {
        let Some(picker) = &mut self.quick_open else {
            return div();
        };
        let now = self.clock.elapsed();
        let reduced = self.reduced_motion || cx.reduce_motion();
        let search_frame = search_bar_frame(
            &mut picker.search_motion,
            picker.search.is_some(),
            now,
            reduced,
        );
        let entry_count = match picker.visible_entries() {
            DirectoryEntries::Ready(entries) => entries.len(),
            _ => 0,
        };
        let listing = picker.listing_motion.sample(now, reduced, entry_count);
        if (search_frame.is_animating() || picker.listing_motion.is_animating())
            && window.visibility().is_visible()
        {
            window.request_animation_frame();
        }
        let picker = self.quick_open.as_ref().expect("browser is open");
        let palette = self.palette;
        let size = window.viewport_size();
        let compact = size.width < COMPACT_WIDTH;
        let width = (size.width - px(if compact { 16. } else { 48. })).min(px(1220.));
        let height = (size.height - px(40.)).min(px(770.));
        let sidebar_width = if compact { 109. } else { 184. };
        let content_width = f32::from(width) - sidebar_width;
        let title = match picker.mode {
            QuickOpenMode::Open => "media.quickOpen.title",
            QuickOpenMode::AppendFiles => "media.quickOpen.appendFiles",
            QuickOpenMode::AppendFolder => "media.quickOpen.appendFolder",
        };
        let close = icon_button("quick-close", "close", palette, false)
            .cursor_pointer()
            .on_click(cx.listener(|view, _, _, cx| view.close_quick_open(cx)));
        let header = div()
            .h(px(52.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .border_b_1()
            .border_color(palette.line)
            .bg(palette.control)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(icon("folder", palette.accent, 18.))
                    .child(self.ui.label(title)),
            )
            .child(quick_tooltip(
                close,
                self.ui.label("media.quickOpen.cancel"),
                palette,
            ));

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
            .on_drop(cx.listener(Self::drop_paths))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| view.close_quick_open(cx)),
            )
            .child(
                div()
                    .w(width)
                    .h(height)
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .occlude()
                    .on_drop(cx.listener(Self::drop_paths))
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
                    .child(self.quick_address_bar(window, cx))
                    .when(search_frame.is_mounted(), |panel| {
                        let target = search_frame.target();
                        panel.child(
                            div()
                                .flex_none()
                                .h(px(target.height_px().unwrap_or_default()))
                                .overflow_hidden()
                                .opacity(target.alpha())
                                .when_some(picker.search.as_ref(), |row, search| {
                                    row.child(self.quick_search_field(search, window, cx))
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .overflow_hidden()
                            .child(self.quick_sidebar(sidebar_width, cx))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w_0()
                                    .min_h_0()
                                    .child(self.quick_toolbar(content_width < 430., cx))
                                    .child(self.quick_contents(content_width, listing, cx)),
                            ),
                    )
                    .child(self.quick_footer(cx)),
            )
            .when_some(picker.context.as_ref(), |el, menu| {
                el.child(self.quick_entry_menu(menu, window, cx))
            })
            .when_some(picker.toolbar_menu.as_ref(), |el, menu| {
                el.child(self.quick_toolbar_menu(menu, window, cx))
            })
    }

    fn quick_search_field(&self, search: &QuickSearch, window: &Window, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let close = icon_button("quick-close-search", "close", palette, false)
            .size(px(27.))
            .cursor_pointer()
            .on_click(cx.listener(|view, _, _, cx| view.close_quick_search(cx)));
        div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(37.))
            .mx_3()
            .my_2()
            .pl_3()
            .pr_1()
            .rounded(px(9.))
            .border_1()
            .border_color(if search.input.read(cx).is_focused(window) {
                palette.accent_border
            } else {
                palette.line
            })
            .bg(palette.control)
            .child(icon("search", palette.accent, 16.))
            .child(input_field(
                &search.input,
                Some(self.ui.label("media.quickOpen.searchPlaceholder")),
                palette,
                cx,
            ))
            .child(quick_tooltip(
                close,
                self.ui.label("media.quickOpen.closeSearch"),
                palette,
            ))
    }

    fn quick_footer(&self, cx: &Context<Self>) -> Div {
        let picker = self.quick_open.as_ref().expect("browser is open");
        let palette = self.palette;
        let (count, folder_readable) = match picker.visible_entries() {
            DirectoryEntries::Ready(entries) => (entries.len(), true),
            _ => (0, false),
        };
        let folder_mode = picker.mode == QuickOpenMode::AppendFolder;
        let loading = picker.navigation.is_some();
        let count_label = match &picker.search {
            Some(search) if search.truncated => "media.quickOpen.searchTruncated",
            _ if picker.showing_search_results() => "media.quickOpen.searchResults",
            _ => "media.quickOpen.items",
        };
        let enabled = !loading
            && if folder_mode {
                folder_readable
            } else {
                !picker.selected.is_empty()
            };
        let action_label = self.ui.label(if folder_mode {
            "media.quickOpen.selectFolder"
        } else {
            "media.quickOpen.openSelected"
        });
        let action = text_button("quick-open-selection", action_label, palette, true).px_4();
        let action = if enabled {
            action
                .cursor_pointer()
                .on_click(cx.listener(|view, _, _, cx| view.open_quick_selection(cx)))
        } else {
            action.opacity(0.42)
        };
        let cancel = text_button(
            "quick-cancel",
            self.ui.label("media.quickOpen.cancel"),
            palette,
            false,
        )
        .cursor_pointer()
        .on_click(cx.listener(|view, _, _, cx| view.close_quick_open(cx)));
        div()
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .bg(palette.control)
            .border_t_1()
            .border_color(palette.line)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_size(px(12.))
                    .text_color(palette.muted)
                    .when(loading && folder_readable, |el| {
                        el.child(self.ui.label("media.quickOpen.loading"))
                    })
                    .when(!loading && folder_readable, |el| {
                        el.child(
                            self.ui
                                .label(count_label)
                                .replace("{count}", &count.to_string()),
                        )
                    })
                    .when(!loading && !picker.selected.is_empty(), |el| {
                        el.child(
                            self.ui
                                .label("media.quickOpen.selected")
                                .replace("{count}", &picker.selected.len().to_string()),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(cancel)
                    .child(action),
            )
    }
}
