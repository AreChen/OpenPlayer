use std::path::Path;

use gpui::{
    Context, Div, ElementId, FontWeight, MouseButton, MouseDownEvent, SharedString, Stateful,
    Window, div, prelude::*, px,
};

use super::super::{PlayerView, QuickLocation, QuickOpenMode, address::address_field};
use super::{SIDEBAR_WIDTH, quick_tooltip};
use crate::windows_gpui::{
    theme::icon,
    widgets::{button_feedback, icon_button, row},
};

impl PlayerView {
    pub(super) fn quick_address_bar(&self, _window: &Window, cx: &Context<Self>) -> Div {
        let picker = self.quick_open.as_ref().expect("browser is open");
        let palette = self.palette;
        let up = if picker.show_media_favorites {
            None
        } else {
            picker.directory.parent().map(Path::to_path_buf)
        };
        let pinned = self
            .quick_favorites()
            .iter()
            .any(|p| same_path(p, &picker.directory));
        let favorite = self.ui.label(if pinned {
            "media.quickOpen.unpin"
        } else {
            "media.quickOpen.pin"
        });
        let path = picker.directory.clone();
        let pin = icon_button("quick-pin", "bookmark", palette, pinned)
            .cursor_pointer()
            .on_click(
                cx.listener(move |view, _, _, cx| view.toggle_quick_favorite(path.clone(), cx)),
            );
        let search = icon_button("quick-find", "search", palette, picker.search.is_some());
        let search = if picker.navigation.is_none() {
            quick_tooltip(search, self.ui.label("media.quickOpen.search"), palette)
                .cursor_pointer()
                .on_click(cx.listener(|view, _, window, cx| view.focus_quick_search(window, cx)))
        } else {
            search.opacity(0.38)
        };

        let address = if let Some(entity) = &picker.address {
            address_field(entity, palette, cx)
        } else if picker.show_media_favorites {
            self.quick_favorites_address(cx)
        } else {
            self.quick_breadcrumbs(&picker.directory, cx)
        };
        let back = self.quick_nav_button(
            "quick-back",
            "arrowLeft",
            "media.quickOpen.back",
            picker.can_go_back(),
            cx,
            |view, cx| view.quick_open_back(cx),
        );
        let forward = self.quick_nav_button(
            "quick-forward",
            "arrowRight",
            "media.quickOpen.forward",
            picker.can_go_forward(),
            cx,
            |view, cx| view.quick_open_forward(cx),
        );
        let parent = self.quick_nav_button(
            "quick-up",
            "arrowUp",
            "media.quickOpen.up",
            up.is_some(),
            cx,
            move |view, cx| {
                if let Some(path) = &up {
                    view.quick_open_directory(path.clone(), cx);
                }
            },
        );

        div()
            .flex_none()
            .border_b_1()
            .border_color(palette.line)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .gap_1()
                            .child(back)
                            .child(forward)
                            .child(parent),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .flex_1()
                            .min_w_0()
                            .h(px(36.))
                            .rounded(px(8.))
                            .border_1()
                            .border_color(if picker.address_unavailable {
                                palette.danger
                            } else if picker.address.is_some() {
                                palette.accent
                            } else {
                                palette.line
                            })
                            .bg(palette.control)
                            .overflow_hidden()
                            .child(address),
                    )
                    .when(!picker.show_media_favorites, |el| {
                        el.child(quick_tooltip(pin, favorite, palette))
                            .child(search)
                    }),
            )
            .when(picker.address_unavailable, |el| {
                el.child(
                    div()
                        .px_4()
                        .pb_2()
                        .text_size(px(12.))
                        .text_color(palette.danger)
                        .child(self.ui.label("media.quickOpen.pathUnavailable")),
                )
            })
    }

    fn quick_favorites_address(&self, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let edit = quick_tooltip(
            icon_button("quick-edit-saved-media-path", "edit", palette, false).size(px(28.)),
            self.ui.label("media.quickOpen.editPath"),
            palette,
        )
        .cursor_pointer()
        .on_click(cx.listener(|view, _, window, cx| view.focus_quick_address(window, cx)));
        div()
            .flex()
            .items_center()
            .flex_1()
            .min_w_0()
            .h_full()
            .pl_2()
            .pr_1()
            .gap_2()
            .child(icon("bookmark", palette.accent, 15.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(13.))
                    .child(self.ui.label("media.quickOpen.mediaFavorites")),
            )
            .child(edit)
    }

    fn quick_nav_button(
        &self,
        id: &'static str,
        glyph: &'static str,
        label: &'static str,
        enabled: bool,
        cx: &Context<Self>,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        let button = icon_button(id, glyph, self.palette, false).w(px(31.));
        if enabled {
            quick_tooltip(button, self.ui.label(label), self.palette)
                .cursor_pointer()
                .on_click(cx.listener(move |view, _, _, cx| on_press(view, cx)))
        } else {
            button.opacity(0.38)
        }
    }

    fn quick_breadcrumbs(&self, path: &Path, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let mut ancestors: Vec<_> = path
            .ancestors()
            .filter(|path| !path.as_os_str().is_empty())
            .collect();
        ancestors.reverse();
        let show_ellipsis = ancestors.len() > 4;
        let recent_start = ancestors.len().saturating_sub(3);
        let crumbs = ancestors
            .into_iter()
            .enumerate()
            .filter_map(|(index, path)| {
                if show_ellipsis && index > 0 && index < recent_start {
                    return if index == 1 {
                        Some(
                            div()
                                .flex_none()
                                .px_1()
                                .text_color(palette.faint)
                                .child("…"),
                        )
                    } else {
                        None
                    };
                }
                let destination = path.to_path_buf();
                let label = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy();
                Some(
                    div()
                        .flex()
                        .min_w_0()
                        .items_center()
                        .flex_shrink_1()
                        .when(index > 0, |el| {
                            el.child(icon("chevronRight", palette.faint, 12.))
                        })
                        .child(button_feedback(
                            div()
                                .id(("quick-crumb", index))
                                .min_w_0()
                                .max_w(px(142.))
                                .truncate()
                                .rounded(px(5.))
                                .px_2()
                                .py_1()
                                .cursor_pointer()
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.quick_open_directory(destination.clone(), cx);
                                    cx.stop_propagation();
                                }))
                                .child(label.into_owned()),
                            palette,
                        )),
                )
            });
        let edit_label = self.ui.label("media.quickOpen.editPath");
        let edit = quick_tooltip(
            icon_button("quick-edit-path", "edit", palette, false).size(px(28.)),
            edit_label.clone(),
            palette,
        )
        .cursor_pointer()
        .on_click(cx.listener(|view, _, window, cx| view.focus_quick_address(window, cx)));
        div()
            .flex()
            .items_center()
            .flex_1()
            .min_w_0()
            .h_full()
            .px_1()
            .child(icon("folder", palette.accent, 15.))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_center()
                    .overflow_hidden()
                    .text_size(px(13.))
                    .children(crumbs)
                    .child(
                        div()
                            .id("quick-edit-path-blank")
                            .flex_1()
                            .min_w(px(8.))
                            .h_full()
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.focus_quick_address(window, cx)
                            })),
                    ),
            )
            .child(edit)
    }

    pub(super) fn quick_sidebar(&self, cx: &Context<Self>) -> Stateful<Div> {
        let picker = self.quick_open.as_ref().expect("browser is open");
        let palette = self.palette;
        let section = |key| {
            div()
                .px_3()
                .pt_2()
                .pb_1()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.muted)
                .child(self.ui.label(key))
        };
        let favorites = self.quick_favorites();
        let drives = self.quick_drives();
        let current = (!picker.show_media_favorites).then_some(picker.directory.as_path());
        div()
            .id("quick-sidebar")
            .w(px(SIDEBAR_WIDTH))
            .min_h_0()
            .flex_none()
            .flex()
            .flex_col()
            .gap_1()
            .overflow_y_scroll()
            .border_r_1()
            .border_color(palette.line)
            .bg(palette.control)
            .p_2()
            .child(section("media.quickOpen.locations"))
            .children(
                self.quick_locations()
                    .iter()
                    .enumerate()
                    .map(|(index, (location, path))| {
                        let glyph = match location {
                            QuickLocation::Home => "home",
                            QuickLocation::Desktop => "tv",
                            QuickLocation::Documents => "document",
                            QuickLocation::Downloads => "download",
                            QuickLocation::Videos => "frames",
                        };
                        self.quick_place(
                            ("quick-location", index),
                            self.ui.label(location.label()),
                            glyph,
                            path,
                            current,
                            cx,
                        )
                    }),
            )
            .child(section("media.quickOpen.favorites"))
            .when(favorites.is_empty(), |el| {
                el.child(
                    div()
                        .px_3()
                        .pb_2()
                        .text_size(px(11.))
                        .text_color(palette.faint)
                        .child(self.ui.label("media.quickOpen.emptyFavorites")),
                )
            })
            .children(
                favorites
                    .iter()
                    .enumerate()
                    .map(|(index, path)| self.quick_pinned_folder(index, path, current, cx)),
            )
            .when(picker.mode != QuickOpenMode::AppendFolder, |el| {
                let selected = picker.show_media_favorites;
                el.child(
                    row("quick-media-favorites", palette, selected)
                        .w_full()
                        .min_h(px(32.))
                        .flex_none()
                        .text_size(px(12.))
                        .cursor_pointer()
                        .child(icon(
                            "bookmark",
                            if selected {
                                palette.accent
                            } else {
                                palette.muted
                            },
                            14.,
                        ))
                        .child(
                            div()
                                .truncate()
                                .child(self.ui.label("media.quickOpen.mediaFavorites")),
                        )
                        .on_click(
                            cx.listener(|view, _, _, cx| view.show_quick_media_favorites(cx)),
                        ),
                )
            })
            .when(!drives.is_empty(), |el| {
                el.child(section("media.quickOpen.drives"))
            })
            .children(drives.iter().enumerate().map(|(index, path)| {
                self.quick_place(
                    ("quick-drive", index),
                    path.to_string_lossy().into_owned().into(),
                    "drive",
                    path,
                    current,
                    cx,
                )
            }))
    }

    fn quick_pinned_folder(
        &self,
        index: usize,
        path: &Path,
        current: Option<&Path>,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let palette = self.palette;
        let selected = current.is_some_and(|current| same_path(path, current));
        let label = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        let path_label = path.to_string_lossy().into_owned().into();
        let destination = path.to_path_buf();
        let context_path = destination.clone();
        let remove_path = destination.clone();
        let open = quick_tooltip(
            div()
                .id(("quick-favorite-open", index))
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .child(icon(
                    "folder",
                    if selected {
                        palette.accent
                    } else {
                        palette.muted
                    },
                    14.,
                ))
                .child(div().truncate().min_w_0().child(label)),
            path_label,
            palette,
        )
        .on_click(
            cx.listener(move |view, _, _, cx| view.quick_open_directory(destination.clone(), cx)),
        );
        let remove = quick_tooltip(
            icon_button(("quick-unpin", index), "close", palette, false)
                .size(px(22.))
                .rounded(px(6.))
                .cursor_pointer()
                .text_color(palette.muted),
            self.ui.label("media.quickOpen.unpin"),
            palette,
        )
        .on_click(cx.listener(move |view, _, _, cx| {
            view.toggle_quick_favorite(remove_path.clone(), cx);
            cx.stop_propagation();
        }));
        row(("quick-favorite", index), palette, selected)
            .w_full()
            .min_h(px(32.))
            .flex_none()
            .pr_2()
            .gap_1()
            .text_size(px(12.))
            .child(open)
            .child(remove)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, _, cx| {
                    view.show_quick_entry_context(context_path.clone(), true, event.position, cx);
                    cx.stop_propagation();
                }),
            )
    }

    fn quick_place(
        &self,
        id: impl Into<ElementId>,
        label: SharedString,
        glyph: &'static str,
        path: &Path,
        current: Option<&Path>,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let palette = self.palette;
        let selected = current.is_some_and(|current| same_path(path, current));
        let item = row(id, palette, selected)
            .w_full()
            .min_h(px(32.))
            .flex_none()
            .gap_2()
            .px_3()
            .text_size(px(12.))
            .cursor_pointer()
            .child(icon(
                glyph,
                if selected {
                    palette.accent
                } else {
                    palette.muted
                },
                14.,
            ))
            .child(div().truncate().min_w_0().flex_1().child(label));
        let tip = path.to_string_lossy().into_owned().into();
        let target = path.to_path_buf();
        let context_path = target.clone();
        quick_tooltip(item, tip, palette)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |view, event: &MouseDownEvent, _, cx| {
                    view.show_quick_entry_context(context_path.clone(), true, event.position, cx);
                    cx.stop_propagation();
                }),
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                view.quick_open_directory(target.clone(), cx);
            }))
    }
}

pub(super) fn same_path(left: &Path, right: &Path) -> bool {
    left == right
        || left
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
}
