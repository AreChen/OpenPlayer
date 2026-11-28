use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context as _, Result, ensure};
use gpui::{Context, Div, div, prelude::*, px};
use libmpv2::Mpv;
use openplayer_plugin_sdk::Permission;
use serde_json::{Value, json};

use super::PlayerView;
use crate::{
    player::{
        locale,
        widgets::{command, heading, item_id, text_button},
    },
    plugins::{HostAction, PlaybackContext, PluginRegistry},
};

#[derive(Default)]
pub(super) struct PluginPanel {
    registry: Option<Arc<PluginRegistry>>,
    busy: bool,
    message: Option<String>,
}

impl PluginPanel {
    pub(super) fn needs_discovery(&self) -> bool {
        self.registry.is_none() && !self.busy
    }
}

impl PlayerView {
    pub(in crate::player) fn plugin_settings(&self, cx: &Context<Self>) -> Div {
        let palette = self.palette;
        let ui = &self.ui;
        let chinese = locale::resolve_language(&ui.language) == "zh-CN";
        let panel = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .items_center()
                    .child(heading(ui.label("settings.nav.plugins"), palette))
                    .child(command(
                        text_button("plugins-folder", ui.label("plugins.folder"), palette, false),
                        "openPluginDirectory",
                        Value::Null,
                        true,
                        cx,
                    ))
                    .child(command(
                        text_button("rescan-plugins", ui.label("plugins.rescan"), palette, false),
                        "reloadPlugins",
                        Value::Null,
                        !self.plugins.busy,
                        cx,
                    )),
            )
            .when(self.plugins.busy, |el| {
                el.child(ui.label("plugins.working"))
            })
            .when_some(self.plugins.message.as_ref(), |el, message| {
                el.child(div().text_color(palette.accent).child(message.clone()))
            });
        let Some(registry) = &self.plugins.registry else {
            return panel;
        };
        panel
            .children(registry.plugins().map(|plugin| {
                let enabled = self.is_plugin_enabled(registry, &plugin.id);
                let permissions = plugin
                    .permissions
                    .iter()
                    .map(|p| {
                        ui.label(match p {
                            Permission::OpenStream => "plugins.openStream",
                            Permission::Capture => "plugins.capture",
                            Permission::SubtitleStyle => "plugins.subtitleStyle",
                        })
                        .to_string()
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .rounded(px(10.))
                    .bg(palette.control)
                    .border_1()
                    .border_color(palette.line)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(heading(plugin.name.get(chinese).to_owned(), palette))
                            .child(command(
                                text_button(
                                    item_id("plugin-enabled", &plugin.id),
                                    ui.label(if enabled {
                                        "plugins.enabled"
                                    } else {
                                        "plugins.disabled"
                                    }),
                                    palette,
                                    enabled,
                                ),
                                "togglePlugin",
                                json!(plugin.id),
                                !self.plugins.busy,
                                cx,
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(palette.muted)
                            .child(format!(
                                "{} · v{} · {}",
                                plugin.id, plugin.version, permissions
                            )),
                    )
                    .when(enabled, |card| {
                        card.child(div().flex().flex_wrap().gap_2().children(
                            plugin.actions.iter().map(|action| {
                                command(
                                    text_button(
                                        item_id(&plugin.id, &action.id),
                                        action.label.get(chinese).to_owned(),
                                        palette,
                                        false,
                                    ),
                                    "runPluginAction",
                                    json!([plugin.id, action.id]),
                                    !self.plugins.busy,
                                    cx,
                                )
                            }),
                        ))
                    })
            }))
            .children(
                registry
                    .diagnostics()
                    .iter()
                    .map(|message| div().child(message.clone())),
            )
    }

    pub(super) fn load_plugins(&mut self, cx: &mut Context<Self>) {
        if self.plugins.busy {
            return;
        }
        self.plugins.busy = true;
        self.plugins.message = None;
        let directory = self.controller.plugin_directory();
        let task = cx
            .background_executor()
            .spawn(async move { PluginRegistry::load(&directory) });
        cx.spawn(async move |view, cx| {
            let registry = task.await;
            let _ = view.update(cx, |view, cx| {
                view.plugins.registry = Some(Arc::new(registry));
                view.plugins.busy = false;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn toggle_plugin(&mut self, id: &str) {
        let Some(registry) = &self.plugins.registry else {
            return;
        };
        let Some(key) = registry.approval_key(id) else {
            return;
        };
        if self.plugins.busy {
            return;
        }
        let enabled = self.is_plugin_enabled(registry, id);
        self.controller.set_plugin_enabled(key, !enabled);
        self.plugins.message = None;
    }

    fn is_plugin_enabled(&self, registry: &PluginRegistry, id: &str) -> bool {
        registry
            .approval_key(id)
            .is_some_and(|key| self.controller.plugin_enabled(key, registry.is_bundled(id)))
    }

    pub(super) fn run_plugin_action(
        &mut self,
        plugin: String,
        action: String,
        cx: &mut Context<Self>,
    ) {
        let Some(registry) = self.plugins.registry.clone() else {
            return;
        };
        if self.plugins.busy || !self.is_plugin_enabled(&registry, &plugin) {
            return;
        }
        let media = self.controller.plugin_media();
        let source = media.as_ref().map(|(_, source)| (*source).to_owned());
        let context = PlaybackContext {
            loaded: self.ui.loaded,
            paused: !self.ui.playing,
            position_ms: (self.ui.position * 1000.) as u64,
            duration_ms: (self.ui.duration * 1000.) as u64,
            subtitle_scale: media
                .and_then(|(mpv, _)| mpv.get_property::<f64>("sub-scale").ok())
                .unwrap_or(1.),
        };
        self.plugins.busy = true;
        self.plugins.message = None;
        let task = cx
            .background_executor()
            .spawn(async move { registry.invoke(&plugin, &action, context) });
        cx.spawn(async move |view, cx| {
            let effect = task.await;
            let _ = view.update(cx, |view, cx| {
                view.plugins.busy = false;
                let result = effect
                    .map_err(anyhow::Error::msg)
                    .and_then(|effect| view.apply_plugin_action(effect, source.as_deref(), cx));
                if let Err(error) = result {
                    view.plugins.message = Some(format!("{error:#}"));
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_plugin_action(
        &mut self,
        effect: HostAction,
        source: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if matches!(effect, HostAction::OpenStreamDialog) {
            self.send("closeSettings", Value::Null, cx);
            self.send("openNetworkStream", Value::Null, cx);
            return Ok(());
        }
        let (mpv, active) = self
            .controller
            .plugin_media()
            .context(self.ui.label("plugins.needsMedia").to_string())?;
        ensure!(
            source == Some(active),
            "{}",
            self.ui.label("plugins.mediaChanged")
        );
        match effect {
            HostAction::SetSubtitleScale { scale } => {
                mpv.set_property("sub-scale", scale)?;
                self.plugins.message = Some(
                    self.ui
                        .label("plugins.scaleSet")
                        .replace("{scale}", &format!("{:.0}%", scale * 100.)),
                );
            }
            HostAction::CaptureScreenshot => {
                ensure!(
                    !self.ui.audio_only,
                    "{}",
                    self.ui.label("plugins.needsVideo")
                );
                let source = mpv.get_property::<String>("path")?;
                self.plugins.busy = true;
                let task = cx.background_executor().spawn(async move {
                    let pictures = dirs::picture_dir()
                        .or_else(|| dirs::home_dir().map(|path| path.join("Pictures")))
                        .context("Unable to locate the Pictures directory")?;
                    save_screenshot(&mpv, &source, &pictures.join("OpenPlayer"))
                });
                cx.spawn(async move |view, cx| {
                    let result = task.await;
                    let _ = view.update(cx, |view, cx| {
                        view.plugins.busy = false;
                        view.plugins.message = Some(match result {
                            Ok(path) => view
                                .ui
                                .label("plugins.saved")
                                .replace("{path}", &path.display().to_string()),
                            Err(error) => format!("{error:#}"),
                        });
                        cx.notify();
                    });
                })
                .detach();
            }
            HostAction::OpenStreamDialog => unreachable!(),
        }
        Ok(())
    }
}

fn save_screenshot(mpv: &Mpv, source: &str, directory: &Path) -> Result<PathBuf> {
    fs::create_dir_all(directory).with_context(|| directory.display().to_string())?;
    let file = tempfile::Builder::new()
        .prefix("OpenPlayer-")
        .suffix(".png")
        .tempfile_in(directory)?
        .into_temp_path();
    ensure!(
        mpv.get_property::<String>("path")? == source,
        "The active media changed"
    );
    mpv.command(
        "screenshot-to-file",
        &[
            file.to_str().context("Invalid screenshot path")?,
            "subtitles",
        ],
    )?;
    ensure!(
        mpv.get_property::<String>("path")? == source && file.metadata()?.len() > 0,
        "Could not capture the current frame"
    );
    Ok(file.keep()?)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn captures_pngs_without_overwriting_or_saving_a_different_video() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let video = directory.path().join("sample.y4m");
        let mut frames = b"YUV4MPEG2 W2 H2 F1:1 Ip A1:1 C420jpeg\n".to_vec();
        frames.extend(b"FRAME\n\x50\x50\x50\x50\x80\x80".repeat(4));
        fs::write(&video, frames)?;
        let mpv = Mpv::with_initializer(|init| {
            init.set_option("vo", "null")?;
            init.set_option("ao", "null")?;
            init.set_option("pause", true)
        })?;
        let source = video.to_str().unwrap();
        mpv.command("loadfile", &[source])?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while mpv.get_property::<i64>("video-out-params/w").is_err() {
            ensure!(Instant::now() < deadline, "video did not load");
            mpv.wait_event(0.02);
        }
        let output = directory.path().join("captures");
        let first = save_screenshot(&mpv, source, &output)?;
        let second = save_screenshot(&mpv, source, &output)?;
        assert_ne!(first, second);
        for path in [first, second] {
            assert!(fs::read(path)?.starts_with(b"\x89PNG\r\n\x1a\n"));
        }
        assert!(save_screenshot(&mpv, "another video", &output).is_err());
        assert_eq!(fs::read_dir(&output)?.count(), 2);
        Ok(())
    }
}
