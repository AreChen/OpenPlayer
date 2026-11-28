#[cfg(windows)]
mod explorer;

use gpui::{
    Context, Div, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px, rgba,
};
use serde_json::{Value, json};

#[cfg(windows)]
pub(super) use explorer::ExplorerPreview;

use super::{
    player_view::{EditTarget, PlayerView},
    theme::{Palette, icon, logo, parse},
    widgets::{command, heading, icon_button, item_id, row, slider, text_button, tooltip},
    window_layout::COMPACT_WIDTH,
};

impl PlayerView {
    pub(super) fn settings_dialog(
        &self,
        section: &str,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        let nav = [
            ("appearance", "palette"),
            ("playback", "play"),
            ("shortcuts", "settings"),
            ("about", "info"),
        ];
        let size = window.viewport_size();
        let compact = size.width < COMPACT_WIDTH;
        let content = match section {
            "playback" => self.playback_settings(cx),
            "shortcuts" => self.shortcut_settings(cx),
            "about" => self.about(cx),
            _ => self.appearance(cx),
        };
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0c0e10a3))
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.send("closeSettings", Value::Null, cx)),
            )
            .child(
                div()
                    .w((size.width - px(if compact { 20. } else { 56. })).min(px(760.)))
                    .h((size.height - px(56.)).min(px(640.)))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .occlude()
                    .rounded(px(14.))
                    .bg(palette.surface)
                    .border_1()
                    .border_color(palette.line)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_5()
                            .py_3()
                            .bg(palette.control)
                            .border_b_1()
                            .border_color(palette.line)
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .text_color(palette.accent)
                                            .child("OpenPlayer"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(23.))
                                            .font_weight(FontWeight::BOLD)
                                            .child(ui.label("settings.title")),
                                    ),
                            )
                            .child(command(
                                tooltip(
                                    icon_button("closeSettings", "close", palette, false),
                                    ui.label("common.close"),
                                    palette,
                                ),
                                "closeSettings",
                                Value::Null,
                                true,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .child(
                                div()
                                    .w(px(if compact { 108. } else { 158. }))
                                    .flex_none()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .bg(palette.strong)
                                    .border_r_1()
                                    .border_color(palette.line)
                                    .children(nav.into_iter().map(|(id, glyph)| {
                                        command(
                                            row(id, palette, id == section)
                                                .gap_2()
                                                .when(!compact, |item| {
                                                    item.child(icon(glyph, palette.accent, 17.))
                                                })
                                                .child(ui.label(&format!("settings.nav.{id}"))),
                                            "settingsSection",
                                            json!(id),
                                            true,
                                            cx,
                                        )
                                    })),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!("settings-{section}")))
                                    .flex_1()
                                    .min_w_0()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .p(px(if compact { 8. } else { 16. }))
                                    .child(content),
                            ),
                    ),
            )
    }

    fn appearance(&self, cx: &Context<Self>) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        let themes = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .children(ui.themes.iter().map(|theme| {
                command(
                    row(
                        item_id("theme", &theme.id),
                        palette,
                        ui.theme_id.as_ref() == Some(&theme.id),
                    )
                    .border_1()
                    .border_color(palette.line)
                    .min_w(px(128.))
                    .min_h(px(42.))
                    .child(
                        div()
                            .size(px(12.))
                            .rounded_full()
                            .bg(parse(&theme.accent).unwrap_or(palette.accent)),
                    )
                    .child(theme.name.clone()),
                    "theme",
                    json!(theme.id),
                    true,
                    cx,
                )
            }));
        let accents = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(self.setting_button(
                "theme-default",
                "settings.appearance.followTheme",
                "accent",
                Value::Null,
                true,
                cx,
            ))
            .children(ui.accents.iter().enumerate().map(|(index, accent)| {
                let active = ui.accent_override.as_ref() == Some(accent);
                let label = ui
                    .label("settings.appearance.useAccent")
                    .replace("{color}", accent);
                command(
                    tooltip(
                        div()
                            .id(("accent", index))
                            .size(px(28.))
                            .flex_none()
                            .rounded_full()
                            .border_1()
                            .border_color(if active { palette.text } else { palette.line })
                            .bg(parse(accent).unwrap_or(palette.accent))
                            .hover(|el| el.border_color(palette.text)),
                        label,
                        palette,
                    ),
                    "accent",
                    json!(accent),
                    true,
                    cx,
                )
            }));
        let languages = div()
            .flex()
            .gap_2()
            .children(ui.languages.iter().map(|language| {
                command(
                    text_button(
                        item_id("language", &language.id),
                        language.label.clone(),
                        palette,
                        ui.language == language.id,
                    ),
                    "language",
                    json!(language.id),
                    true,
                    cx,
                )
            }));
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(heading(ui.label("settings.appearance.title"), palette))
                    .child(self.setting_button(
                        "appearance-reset",
                        "common.reset",
                        "resetAppearance",
                        Value::Null,
                        true,
                        cx,
                    )),
            )
            .child(themes)
            .child(
                card(palette)
                    .child(heading(ui.label("settings.appearance.accent"), palette))
                    .child(accents)
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .child(self.draft_field(
                                EditTarget::Accent,
                                ui.label("settings.appearance.freePick"),
                                ui.accent_override.as_deref().unwrap_or(""),
                                cx,
                            ))
                            .child(
                                text_button(
                                    "apply-accent",
                                    ui.label("common.choose"),
                                    palette,
                                    false,
                                )
                                .cursor_pointer()
                                .on_click(cx.listener(
                                    |view, _, _, cx| {
                                        if matches!(view.edit, Some((EditTarget::Accent, _))) {
                                            view.commit_edit(cx);
                                        }
                                    },
                                )),
                            ),
                    ),
            )
            .child(
                card(palette)
                    .child(heading(ui.label("settings.appearance.chrome"), palette))
                    .child(self.opacity_setting("titlebarOpacity", ui.titlebar_opacity, cx))
                    .child(self.opacity_setting("transportOpacity", ui.transport_opacity, cx)),
            )
            .child(
                card(palette)
                    .child(heading(ui.label("settings.appearance.language"), palette))
                    .child(
                        div()
                            .text_color(palette.muted)
                            .child(ui.label("settings.appearance.languageDescription")),
                    )
                    .child(languages),
            )
    }

    fn opacity_setting(&self, key: &'static str, opacity: f64, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().child(self.ui.label(&format!("settings.appearance.{key}"))))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(slider(key, opacity, 1., palette, cx))
                    .child(
                        div()
                            .w(px(40.))
                            .flex_none()
                            .text_right()
                            .text_color(palette.muted)
                            .child(format!("{:.0}%", opacity * 100.)),
                    ),
            )
    }

    fn playback_settings(&self, cx: &Context<Self>) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        let options = [
            (
                "settings.playback.incognito",
                "settings.playback.incognitoDescription",
                "incognito",
                ui.incognito,
            ),
            (
                "settings.playback.quietKeyboard",
                "settings.playback.quietKeyboardDescription",
                "quietKeyboard",
                ui.quiet_keyboard,
            ),
        ];
        let panel = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(heading(ui.label("settings.playback.title"), palette));
        #[cfg(windows)]
        let panel = panel.child(self.default_app_settings(cx));
        let panel = panel
            .children(
                options
                    .into_iter()
                    .map(|(label, description, kind, enabled)| {
                        card(palette)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().flex_1().child(heading(ui.label(label), palette)))
                                    .child(command(
                                        text_button(
                                            kind,
                                            ui.label(if enabled {
                                                "common.enabled"
                                            } else {
                                                "common.disabled"
                                            }),
                                            palette,
                                            enabled,
                                        ),
                                        kind,
                                        json!(!enabled),
                                        true,
                                        cx,
                                    )),
                            )
                            .child(div().text_color(palette.muted).child(ui.label(description)))
                    }),
            )
            .child(self.setting_button(
                "clear-playback-history",
                "settings.playback.clearHistory",
                "clearHistory",
                Value::Null,
                !ui.history.is_empty(),
                cx,
            ));
        #[cfg(windows)]
        let panel = panel.child(self.explorer_settings(cx));
        panel
    }

    fn shortcut_settings(&self, cx: &Context<Self>) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(heading(ui.label("settings.shortcuts.title"), palette))
                    .child(self.setting_button(
                        "reset-shortcuts",
                        "common.restoreDefaults",
                        "resetShortcuts",
                        Value::Null,
                        true,
                        cx,
                    )),
            )
            .children(ui.shortcut_rows.iter().map(|shortcut| {
                let active = ui.recording_shortcut.as_ref() == Some(&shortcut.id);
                let label = if active {
                    ui.label("common.inputting")
                } else {
                    shortcut.binding.as_ref().map_or_else(
                        || ui.label("common.unset"),
                        |binding| binding.clone().into(),
                    )
                };
                card(palette)
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(div().flex_1().truncate().child(shortcut.label.clone()))
                    .child(command(
                        tooltip(
                            text_button(item_id("shortcut", &shortcut.id), label, palette, active),
                            ui.label(if active {
                                "common.cancel"
                            } else {
                                "settings.shortcuts.change"
                            }),
                            palette,
                        ),
                        "recordShortcut",
                        if active {
                            Value::Null
                        } else {
                            json!(shortcut.id)
                        },
                        true,
                        cx,
                    ))
            }))
    }

    fn about(&self, cx: &Context<Self>) -> Div {
        let ui = &self.ui;
        let palette = self.palette;
        let mut body = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(heading(ui.label("settings.about.title"), palette))
            .child(
                card(palette)
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(logo(palette.accent, 68.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(heading(
                                ui.version.as_ref().map_or_else(
                                    || "OpenPlayer".into(),
                                    |version| version.name.clone(),
                                ),
                                palette,
                            ))
                            .child(
                                div()
                                    .text_color(palette.muted)
                                    .child(ui.label("settings.about.description")),
                            ),
                    ),
            );
        if let Some(version) = ui.version.as_ref() {
            body = body
                .child(
                    card(palette)
                        .child(format!(
                            "{}  {}",
                            ui.label("settings.about.version"),
                            version.version
                        ))
                        .child(format!(
                            "{}  {}",
                            ui.label("settings.about.license"),
                            version.license
                        )),
                )
                .child(self.release_settings(cx))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(self.setting_button(
                            "repo-link",
                            "settings.about.repository",
                            "externalUrl",
                            json!(version.repository),
                            true,
                            cx,
                        ))
                        .child(self.setting_button(
                            "releases-link",
                            "settings.about.releases",
                            "externalUrl",
                            json!(version.releases_url),
                            true,
                            cx,
                        )),
                );
        }
        body
    }

    fn release_settings(&self, cx: &Context<Self>) -> Div {
        use super::updates::UpdateStatus;

        let ui = &self.ui;
        let status = &self.update_status;
        let checking = matches!(status, UpdateStatus::Checking);
        let message = match status {
            UpdateStatus::Idle => ui.label("settings.about.updateIdle"),
            UpdateStatus::Checking => ui.label("settings.about.checkingUpdates"),
            UpdateStatus::Current => ui.label("settings.about.upToDate"),
            UpdateStatus::Available(update, None) => ui
                .label("settings.about.updateAvailable")
                .replace("{version}", &update.version)
                .into(),
            UpdateStatus::Available(_, Some(error)) => ui
                .label("settings.about.updateDownloadFailed")
                .replace("{message}", error)
                .into(),
            UpdateStatus::Downloading => ui.label("settings.about.downloadingUpdate"),
            UpdateStatus::Ready(version, _, None) => ui
                .label(if cfg!(unix) {
                    "settings.about.updateReadyNative"
                } else {
                    "settings.about.updateReady"
                })
                .replace("{version}", version)
                .into(),
            UpdateStatus::Ready(_, _, Some(error)) => ui
                .label("settings.about.updateInstallFailed")
                .replace("{message}", error)
                .into(),
            #[cfg(target_os = "linux")]
            UpdateStatus::Installed(_) => ui.label("settings.about.updateInstalled"),
            #[cfg(target_os = "macos")]
            UpdateStatus::Opened(_) => ui.label("settings.about.updateInstallerOpened"),
            UpdateStatus::Installing => ui.label(if cfg!(unix) {
                "settings.about.installingNativeUpdate"
            } else {
                "settings.about.installingUpdate"
            }),
            UpdateStatus::Failed(error) => ui
                .label("settings.about.updateCheckFailed")
                .replace("{message}", error)
                .into(),
        };
        card(self.palette)
            .child(heading(ui.label("settings.about.update"), self.palette))
            .child(div().text_color(self.palette.muted).child(message))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(self.setting_button(
                        "check-for-updates",
                        if checking {
                            "settings.about.checkingShort"
                        } else {
                            "settings.about.checkForUpdates"
                        },
                        "checkForUpdates",
                        Value::Null,
                        status.can_check(),
                        cx,
                    ))
                    .when(matches!(status, UpdateStatus::Available(..)), |row| {
                        row.child(self.setting_button(
                            "download-update",
                            "settings.about.downloadUpdate",
                            "downloadUpdate",
                            Value::Null,
                            true,
                            cx,
                        ))
                    })
                    .when(status.restart_path().is_some(), |row| {
                        row.child(self.setting_button(
                            "restart-update",
                            "settings.about.restartUpdate",
                            "restartUpdate",
                            Value::Null,
                            true,
                            cx,
                        ))
                    })
                    .when(matches!(status, UpdateStatus::Ready(..)), |row| {
                        row.child(self.setting_button(
                            "install-update",
                            if cfg!(target_os = "macos") {
                                "settings.about.openInstaller"
                            } else {
                                "settings.about.installUpdate"
                            },
                            "installUpdate",
                            Value::Null,
                            true,
                            cx,
                        ))
                    }),
            )
    }

    fn setting_button(
        &self,
        id: &'static str,
        label: &str,
        intent: &'static str,
        value: Value,
        enabled: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        command(
            text_button(id, self.ui.label(label), self.palette, false),
            intent,
            value,
            enabled,
            cx,
        )
    }
}

fn card(palette: Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .rounded(px(10.))
        .p_3()
        .bg(palette.control)
        .border_1()
        .border_color(palette.line)
}
