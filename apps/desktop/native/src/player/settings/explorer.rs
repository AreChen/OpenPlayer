use std::collections::HashSet;

use gpui::{Context, Div, ElementId, SharedString, Stateful, div, prelude::*};

use crate::shell_preview::{
    ShellPreviewDefaultStatus, ShellPreviewFormatInfo, shell_preview_default_status,
    shell_preview_formats, shell_preview_open_default_apps_settings,
    shell_preview_register_formats,
};

use super::card;
use crate::player::{
    player_view::PlayerView,
    widgets::{heading, item_id, text_button},
};

pub(in crate::player) struct ExplorerPreview {
    formats: Vec<ShellPreviewFormatInfo>,
    selected: HashSet<&'static str>,
    registering: bool,
    status: Option<Status>,
    defaults: Option<ShellPreviewDefaultStatus>,
    checking_defaults: bool,
    default_apps_error: Option<String>,
}

enum Status {
    Registered(usize),
    RegisterFailed(String),
}

#[derive(Clone, Copy)]
enum Action {
    Toggle(&'static str),
    ToggleAll,
    RestoreDefaults,
    Register,
    DefaultApps,
    RefreshDefaultApps,
}

impl ExplorerPreview {
    pub(in crate::player) fn new() -> Self {
        let formats = shell_preview_formats();
        Self {
            selected: common_formats(&formats),
            formats,
            registering: false,
            status: None,
            defaults: None,
            checking_defaults: false,
            default_apps_error: None,
        }
    }

    fn all_selected(&self) -> bool {
        self.selected.len() == self.formats.len()
    }
}

fn common_formats(formats: &[ShellPreviewFormatInfo]) -> HashSet<&'static str> {
    formats
        .iter()
        .filter(|format| format.common)
        .map(|format| format.extension)
        .collect()
}

impl PlayerView {
    pub(in crate::player) fn default_app_settings(&self, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let preview = &self.explorer_preview;
        let status = if preview.checking_defaults {
            self.explorer_label("checkingDefaults")
        } else if let Some(defaults) = &preview.defaults {
            self.explorer_label("defaultCoverage")
                .replace("{active}", &defaults.openplayer_count.to_string())
                .replace("{total}", &defaults.total_count.to_string())
                .into()
        } else {
            self.explorer_label("defaultCoveragePending")
        };
        card(palette)
            .child(heading(self.explorer_label("defaultPlayerTitle"), palette))
            .child(
                div()
                    .text_color(palette.muted)
                    .child(self.explorer_label("defaultPlayerDescription")),
            )
            .child(div().text_color(palette.text).child(status))
            .when_some(
                preview
                    .defaults
                    .as_ref()
                    .filter(|_| !preview.checking_defaults),
                |el, defaults| {
                    let details = self
                        .explorer_label("defaultCoverageDetails")
                        .replace("{other}", &defaults.other_count.to_string())
                        .replace("{unassigned}", &defaults.unassigned_count.to_string());
                    el.child(div().text_color(palette.muted).child(details))
                        .when(defaults.unknown_count != 0, |el| {
                            el.child(
                                div().text_color(palette.muted).child(
                                    self.explorer_label("defaultCoverageUnknown")
                                        .replace("{count}", &defaults.unknown_count.to_string()),
                                ),
                            )
                        })
                },
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(self.explorer_button(
                        "explorer-default-apps",
                        self.explorer_label("defaultApps"),
                        true,
                        Action::DefaultApps,
                        cx,
                    ))
                    .child(self.explorer_button(
                        "explorer-refresh-defaults",
                        self.explorer_label("refreshDefaults"),
                        false,
                        Action::RefreshDefaultApps,
                        cx,
                    )),
            )
            .when_some(preview.default_apps_error.as_ref(), |el, error| {
                el.child(
                    div().text_color(palette.danger).child(
                        self.explorer_label("openDefaultAppsFailed")
                            .replace("{error}", error),
                    ),
                )
            })
    }

    pub(in crate::player) fn refresh_default_app_status(&mut self, cx: &mut Context<Self>) {
        if self.explorer_preview.checking_defaults {
            return;
        }
        self.explorer_preview.checking_defaults = true;
        let query = cx
            .background_executor()
            .spawn(async { shell_preview_default_status() });
        cx.spawn(async move |view, cx| {
            let status = query.await;
            let _ = view.update(cx, |view, cx| {
                view.explorer_preview.defaults = Some(status);
                view.explorer_preview.checking_defaults = false;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn explorer_settings(&self, cx: &Context<Self>) -> Div {
        let preview = &self.explorer_preview;
        let palette = self.palette;
        let count = preview.selected.len();
        let register_label = if preview.registering {
            self.explorer_label("registering")
        } else {
            self.explorer_label("register")
                .replace("{count}", &count.to_string())
                .into()
        };
        let status = preview.status.as_ref().map(|status| match status {
            Status::Registered(count) => self
                .explorer_label("registered")
                .replace("{count}", &count.to_string()),
            Status::RegisterFailed(error) => {
                self.explorer_label("failed").replace("{error}", error)
            }
        });
        card(palette)
            .child(heading(self.explorer_label("title"), palette))
            .child(
                div()
                    .text_color(palette.muted)
                    .child(self.explorer_label("description")),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(self.explorer_button(
                        "explorer-toggle-all",
                        self.explorer_label(if preview.all_selected() {
                            "clearAll"
                        } else {
                            "selectAll"
                        }),
                        false,
                        Action::ToggleAll,
                        cx,
                    ))
                    .child(self.explorer_button(
                        "explorer-defaults",
                        self.explorer_label("defaults"),
                        false,
                        Action::RestoreDefaults,
                        cx,
                    ))
                    .child(self.explorer_button(
                        "explorer-register",
                        register_label,
                        count != 0,
                        Action::Register,
                        cx,
                    )),
            )
            .when_some(status, |el, status| {
                el.child(div().text_color(palette.muted).child(status))
            })
            .children(["video", "audio"].map(|kind| {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(heading(self.explorer_label(kind), palette))
                    .child(
                        div().flex().flex_wrap().gap_1().children(
                            preview
                                .formats
                                .iter()
                                .filter(|format| format.kind == kind)
                                .map(|format| {
                                    self.explorer_button(
                                        item_id("explorer-format", format.extension),
                                        format!(".{}", format.extension).into(),
                                        preview.selected.contains(format.extension),
                                        Action::Toggle(format.extension),
                                        cx,
                                    )
                                }),
                        ),
                    )
            }))
    }

    fn explorer_label(&self, key: &str) -> SharedString {
        self.ui.label(&format!("settings.shellPreview.{key}"))
    }

    fn explorer_button(
        &self,
        id: impl Into<ElementId>,
        label: SharedString,
        active: bool,
        action: Action,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let button = text_button(id, label, self.palette, active);
        let preview = &self.explorer_preview;
        let enabled = match action {
            Action::DefaultApps => true,
            Action::RefreshDefaultApps => !preview.checking_defaults,
            Action::Register => !preview.registering && !preview.selected.is_empty(),
            _ => !preview.registering,
        };
        if !enabled {
            button.opacity(0.42)
        } else {
            button
                .cursor_pointer()
                .on_click(cx.listener(move |view, _, _, cx| view.explorer_action(action, cx)))
        }
    }

    fn explorer_action(&mut self, action: Action, cx: &mut Context<Self>) {
        let preview = &mut self.explorer_preview;
        match action {
            Action::Toggle(extension) => {
                preview.status = None;
                if !preview.selected.remove(extension) {
                    preview.selected.insert(extension);
                }
            }
            Action::ToggleAll => {
                preview.status = None;
                if preview.all_selected() {
                    preview.selected.clear();
                } else {
                    preview.selected = preview
                        .formats
                        .iter()
                        .map(|format| format.extension)
                        .collect();
                }
            }
            Action::RestoreDefaults => {
                preview.status = None;
                preview.selected = common_formats(&preview.formats);
            }
            Action::DefaultApps => {
                preview.default_apps_error = shell_preview_open_default_apps_settings().err();
            }
            Action::RefreshDefaultApps => self.refresh_default_app_status(cx),
            Action::Register => {
                preview.status = None;
                preview.registering = true;
                let selected = preview
                    .selected
                    .iter()
                    .map(|value| (*value).into())
                    .collect();
                let registration = cx
                    .background_executor()
                    .spawn(async move { shell_preview_register_formats(selected) });
                cx.spawn(async move |view, cx| {
                    let result = registration.await;
                    let _ = view.update(cx, |view, cx| {
                        let preview = &mut view.explorer_preview;
                        preview.registering = false;
                        preview.status = Some(match result {
                            Ok(summary) => Status::Registered(summary.registered_count),
                            Err(error) => Status::RegisterFailed(error),
                        });
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }
}
