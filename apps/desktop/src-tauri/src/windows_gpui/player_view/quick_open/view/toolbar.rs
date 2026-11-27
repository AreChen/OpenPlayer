use gpui::{
    Context, Div, ElementId, MouseButton, Pixels, Point, Rgba, SharedString, Stateful, Window, div,
    prelude::*, px,
};

use super::{
    super::{
        PlayerView, QuickOpenMode,
        listing::{Category, SortBy},
    },
    quick_tooltip,
};
use crate::windows_gpui::{
    theme::icon,
    widgets::{icon_button_box, row},
};

#[derive(Clone, Copy)]
pub(in crate::windows_gpui::player_view::quick_open) struct ToolbarMenu {
    kind: MenuKind,
    anchor: Point<Pixels>,
}

#[derive(Clone, Copy)]
enum MenuKind {
    Sort,
    Category,
}

#[derive(Clone, Copy)]
enum Choice {
    SortBy(SortBy),
    Descending(bool),
    Category(Category),
}

impl Choice {
    fn label(self) -> &'static str {
        match self {
            Self::SortBy(SortBy::Name) => "media.quickOpen.sortName",
            Self::SortBy(SortBy::Type) => "media.quickOpen.sortType",
            Self::SortBy(SortBy::Modified) => "media.quickOpen.sortModified",
            Self::SortBy(SortBy::Size) => "media.quickOpen.sortSize",
            Self::Descending(false) => "media.quickOpen.ascending",
            Self::Descending(true) => "media.quickOpen.descending",
            Self::Category(Category::All) => "media.quickOpen.categoryAll",
            Self::Category(Category::Video) => "media.quickOpen.categoryVideo",
            Self::Category(Category::Audio) => "media.quickOpen.categoryAudio",
            Self::Category(Category::Folders) => "media.quickOpen.categoryFolders",
        }
    }
}

impl PlayerView {
    pub(super) fn quick_toolbar(&self, compact: bool, cx: &Context<Self>) -> Div {
        let picker = self.quick_open.as_ref().expect("browser is open");
        let sort_label: SharedString = format!(
            "{} · {}",
            self.ui.label("media.quickOpen.sort"),
            self.ui.label(Choice::SortBy(picker.sort.by).label())
        )
        .into();
        let category_label: SharedString = format!(
            "{} · {}",
            self.ui.label("media.quickOpen.category"),
            self.ui
                .label(Choice::Category(picker.filter.category).label())
        )
        .into();
        let palette = self.palette;
        let button =
            |id, glyph, label, active| self.quick_toolbar_button(id, glyph, label, active, compact);
        let only_media = self
            .quick_toolbar_button(
                "quick-only-media",
                "filter",
                self.ui.label("media.quickOpen.onlyMedia"),
                picker.filter.only_media,
                false,
            )
            .on_click(cx.listener(|view, _, _, cx| {
                if let Some(picker) = &mut view.quick_open {
                    picker.filter.only_media = !picker.filter.only_media;
                    view.refresh_quick_listing(true, cx);
                }
            }));
        let sort = button("quick-sort", "sort", sort_label, false).on_click(cx.listener(
            |view, event: &gpui::ClickEvent, _, cx| {
                view.open_quick_toolbar(MenuKind::Sort, event.position(), cx);
            },
        ));
        let category = button(
            "quick-category",
            "category",
            category_label,
            picker.filter.category != Category::All,
        )
        .on_click(cx.listener(|view, event: &gpui::ClickEvent, _, cx| {
            view.open_quick_toolbar(MenuKind::Category, event.position(), cx);
        }));
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.))
            .px_3()
            .py_1()
            .border_b_1()
            .border_color(palette.line)
            .child(sort)
            .when(picker.mode != QuickOpenMode::AppendFolder, |el| {
                el.child(category)
                    .when(!picker.show_media_favorites, |el| el.child(only_media))
            })
    }

    fn quick_toolbar_button(
        &self,
        id: impl Into<ElementId>,
        glyph: &'static str,
        label: SharedString,
        active: bool,
        compact: bool,
    ) -> Stateful<Div> {
        let palette = self.palette;
        let button = icon_button_box(id, palette, active)
            .h(px(30.))
            .border_color(if active {
                palette.accent_border
            } else {
                palette.line
            })
            .text_size(px(12.))
            .cursor_pointer()
            .child(icon(glyph, palette.accent, 15.))
            .when(!compact, |el| {
                el.w_auto()
                    .px_2()
                    .gap(px(5.))
                    .text_color(if active { palette.accent } else { palette.text })
                    .child(label.clone())
            });
        quick_tooltip(button, label, palette)
    }

    fn open_quick_toolbar(
        &mut self,
        kind: MenuKind,
        anchor: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let Some(picker) = &mut self.quick_open {
            picker.context = None;
            picker.toolbar_menu = Some(ToolbarMenu { kind, anchor });
            cx.notify();
        }
    }

    fn close_quick_toolbar(&mut self, cx: &mut Context<Self>) {
        if let Some(picker) = &mut self.quick_open {
            picker.toolbar_menu = None;
            cx.notify();
        }
    }

    pub(super) fn quick_toolbar_menu(
        &self,
        menu: &ToolbarMenu,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let picker = self.quick_open.as_ref().expect("browser is open");
        let palette = self.palette;
        let sort = [
            Choice::SortBy(SortBy::Name),
            Choice::SortBy(SortBy::Type),
            Choice::SortBy(SortBy::Modified),
            Choice::SortBy(SortBy::Size),
            Choice::Descending(false),
            Choice::Descending(true),
        ];
        let categories = [
            Choice::Category(Category::All),
            Choice::Category(Category::Video),
            Choice::Category(Category::Audio),
            Choice::Category(Category::Folders),
        ];
        let options: &[Choice] = match menu.kind {
            MenuKind::Sort => &sort,
            MenuKind::Category => &categories,
        };
        let viewport = window.viewport_size();
        let width = px(172.);
        let height = px(options.len() as f32 * 31. + 14.);
        let left = (menu.anchor.x - px(10.))
            .min(viewport.width - width - px(8.))
            .max(px(8.));
        let top = (menu.anchor.y + px(17.))
            .min(viewport.height - height - px(8.))
            .max(px(8.));
        let dismiss = cx.listener(|view, _, _, cx| {
            view.close_quick_toolbar(cx);
            cx.stop_propagation();
        });
        div()
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(MouseButton::Left, dismiss)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|view, _, _, cx| {
                    view.close_quick_toolbar(cx);
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .absolute()
                    .left(left)
                    .top(top)
                    .w(width)
                    .p_1()
                    .gap_1()
                    .bg(Rgba {
                        a: 1.,
                        ..palette.strong
                    })
                    .border_1()
                    .border_color(palette.line)
                    .rounded(px(12.))
                    .shadow_lg()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .children(options.iter().copied().enumerate().map(|(index, choice)| {
                        let active = match choice {
                            Choice::SortBy(by) => picker.sort.by == by,
                            Choice::Descending(descending) => picker.sort.descending == descending,
                            Choice::Category(category) => picker.filter.category == category,
                        };
                        div()
                            .when(matches!(choice, Choice::Descending(false)), |el| {
                                el.child(div().mx_2().my_1().h(px(1.)).bg(palette.line))
                            })
                            .child(
                                row(("quick-toolbar-option", index), palette, active)
                                    .min_h(px(30.))
                                    .text_size(px(13.))
                                    .justify_between()
                                    .cursor_pointer()
                                    .child(self.ui.label(choice.label()))
                                    .when(active, |el| el.child(icon("check", palette.accent, 14.)))
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        view.select_quick_toolbar(choice, cx);
                                        cx.stop_propagation();
                                    })),
                            )
                    })),
            )
    }

    fn select_quick_toolbar(&mut self, choice: Choice, cx: &mut Context<Self>) {
        let Some(picker) = &mut self.quick_open else {
            return;
        };
        picker.toolbar_menu = None;
        let filter_changed = match choice {
            Choice::SortBy(by) => {
                picker.sort.by = by;
                false
            }
            Choice::Descending(descending) => {
                picker.sort.descending = descending;
                false
            }
            Choice::Category(category) => {
                let changed = picker.filter.category != category;
                picker.filter.category = category;
                changed
            }
        };
        self.refresh_quick_listing(filter_changed, cx);
    }
}
