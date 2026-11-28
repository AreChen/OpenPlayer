use std::path::PathBuf;

use gpui::{Context, Div, MouseButton, Pixels, Point, Rgba, Window, div, prelude::*, px};

use super::super::{EntryKind, PlayerView};
use crate::player::{theme::icon, widgets::row};

pub(in crate::player::player_view::quick_open) struct QuickEntryContext {
    path: PathBuf,
    kind: EntryKind,
    position: Point<Pixels>,
}

#[derive(Clone, Copy)]
enum Action {
    Play,
    EnterFolder,
    ToggleFolderFavorite,
    ToggleMediaFavorite,
    Reveal,
    MediaProperties,
}

impl PlayerView {
    pub(super) fn show_quick_entry_context(
        &mut self,
        path: PathBuf,
        kind: EntryKind,
        position: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if let Some(picker) = &mut self.quick_open {
            picker.toolbar_menu = None;
            picker.context = Some(QuickEntryContext {
                path,
                kind,
                position,
            });
            cx.notify();
        }
    }

    fn close_quick_entry_context(&mut self, cx: &mut Context<Self>) {
        if let Some(picker) = &mut self.quick_open {
            picker.context = None;
            cx.notify();
        }
    }

    pub(super) fn quick_entry_menu(
        &self,
        menu: &QuickEntryContext,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let palette = self.palette;
        let reveal = (Action::Reveal, "folder", "media.quickOpen.revealDirectory");
        let other_files = [reveal];
        let files = [
            (Action::Play, "play", "media.quickOpen.playMedia"),
            (
                Action::ToggleMediaFavorite,
                "bookmark",
                if self.quick_media_is_favorite(&menu.path) {
                    "media.quickOpen.unfavoriteMedia"
                } else {
                    "media.quickOpen.favoriteMedia"
                },
            ),
            reveal,
            (
                Action::MediaProperties,
                "info",
                "media.quickOpen.properties",
            ),
        ];
        let directories = [
            (Action::EnterFolder, "folder", "media.quickOpen.openFolder"),
            (
                Action::ToggleFolderFavorite,
                "bookmark",
                if self
                    .quick_favorites()
                    .iter()
                    .any(|path| super::navigation::same_path(path, &menu.path))
                {
                    "media.quickOpen.unpin"
                } else {
                    "media.quickOpen.pin"
                },
            ),
            reveal,
        ];
        let actions: &[(Action, &str, &str)] = match menu.kind {
            EntryKind::Folder => &directories,
            EntryKind::Other => &other_files,
            EntryKind::Video | EntryKind::Audio => &files,
        };
        let size = window.viewport_size();
        let width = px(196.);
        let height = px(12. + actions.len() as f32 * 33.);
        let left = menu.position.x.min(size.width - width - px(8.)).max(px(8.));
        let top = menu
            .position
            .y
            .min(size.height - height - px(8.))
            .max(px(8.));
        let path = &menu.path;
        div()
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| {
                    view.close_quick_entry_context(cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|view, _, _, cx| {
                    view.close_quick_entry_context(cx);
                    cx.stop_propagation();
                }),
            )
            .child(
                div()
                    .absolute()
                    .left(left)
                    .top(top)
                    .w(width)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .p_1()
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
                    .children(actions.iter().enumerate().map(
                        |(index, &(action, glyph, label))| {
                            let path = path.clone();
                            row(("quick-entry-action", index), palette, false)
                                .gap_2()
                                .text_size(px(13.))
                                .cursor_pointer()
                                .child(icon(glyph, palette.accent, 14.))
                                .child(self.ui.label(label))
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.close_quick_entry_context(cx);
                                    match action {
                                        Action::Play => view.play_quick_entry(path.clone(), cx),
                                        Action::EnterFolder => {
                                            view.quick_open_directory(path.clone(), cx)
                                        }
                                        Action::ToggleFolderFavorite => {
                                            view.toggle_quick_favorite(path.clone(), cx)
                                        }
                                        Action::ToggleMediaFavorite => {
                                            view.toggle_quick_media_favorite(path.clone(), cx)
                                        }
                                        Action::Reveal => view.reveal_quick_entry(&path, cx),
                                        Action::MediaProperties => {
                                            view.open_quick_media_properties(path.clone(), cx)
                                        }
                                    }
                                    cx.stop_propagation();
                                }))
                        },
                    )),
            )
    }
}
