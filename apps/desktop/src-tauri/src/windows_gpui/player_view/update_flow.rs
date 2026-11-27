use std::{mem, time::Duration};

use anyhow::Context as _;
use gpui::Context;

use super::PlayerView;
use crate::windows_gpui::updates::{self, SmokeAction, UpdateStatus};

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(15);
const CHECK_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

impl PlayerView {
    pub(super) fn start_update_checks(&mut self, cx: &mut Context<Self>) {
        if let Some(smoke) = &self.update_smoke {
            smoke.record(&format!("started:{}", env!("CARGO_PKG_VERSION")));
            if !smoke.wait_for_playback && smoke.action != SmokeAction::Playback {
                self.check_for_updates(cx);
            }
            return;
        }
        let executor = cx.background_executor().clone();
        cx.spawn(async move |view, cx| {
            let mut delay = FIRST_CHECK_DELAY;
            loop {
                executor.timer(delay).await;
                if view.update(cx, PlayerView::check_for_updates).is_err() {
                    break;
                }
                delay = CHECK_INTERVAL;
            }
        })
        .detach();
    }

    pub(super) fn report_smoke_playback(&mut self, cx: &mut Context<Self>) {
        let Some(smoke) = self.update_smoke.as_ref().filter(|_| !self.smoke_playing) else {
            return;
        };
        if self.ui.advancing && self.ui.position > 0.1 {
            self.smoke_playing = true;
            smoke.record("playing");
            if smoke.action != SmokeAction::Playback && smoke.wait_for_playback {
                self.check_for_updates(cx);
            }
        } else if let Some(error) = &self.ui.error {
            smoke.record(&format!("error:{error}"));
            self.smoke_playing = true;
        }
    }

    pub(super) fn check_for_updates(&mut self, cx: &mut Context<Self>) {
        if !self.update_status.can_check() {
            return;
        }
        self.update_status = UpdateStatus::Checking;
        cx.notify();
        let check = cx.background_executor().spawn(async { updates::check() });
        cx.spawn(async move |view, cx| {
            let result = check.await;
            let _ = view.update(cx, |view, cx| {
                let smoke = view.update_smoke.as_ref();
                view.update_status = match result {
                    Ok(Some(update)) => {
                        if let Some(smoke) = smoke {
                            smoke.record(&format!("available:{}", update.version));
                        }
                        UpdateStatus::Available(update, None)
                    }
                    Ok(None) => {
                        if let Some(smoke) = smoke {
                            smoke.record("current");
                        }
                        UpdateStatus::Current
                    }
                    Err(error) => {
                        if let Some(smoke) = smoke {
                            smoke.record(&format!("error:{error:#}"));
                        }
                        UpdateStatus::Failed(format!("{error:#}"))
                    }
                };
                if smoke.is_some_and(|smoke| smoke.action != SmokeAction::Playback) {
                    view.download_update(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn download_update(&mut self, cx: &mut Context<Self>) {
        let UpdateStatus::Available(update, _) = &self.update_status else {
            return;
        };
        let update = update.clone();
        self.update_status = UpdateStatus::Downloading;
        cx.notify();
        let download = cx.background_executor().spawn(async move {
            let result = updates::download(&update);
            (update, result)
        });
        cx.spawn(async move |view, cx| {
            let (update, result) = download.await;
            let _ = view.update(cx, |view, cx| {
                let smoke = view.update_smoke.as_ref();
                view.update_status = match result {
                    Ok(installer) => {
                        if let Some(smoke) = smoke {
                            smoke.record(&format!("verified:{}", update.version));
                        }
                        UpdateStatus::Ready(update.version, installer, None)
                    }
                    Err(error) => {
                        if let Some(smoke) = smoke {
                            smoke.record(&format!("error:{error:#}"));
                        }
                        UpdateStatus::Available(update, Some(format!("{error:#}")))
                    }
                };
                if smoke.is_some_and(|smoke| {
                    matches!(
                        smoke.action,
                        SmokeAction::Install | SmokeAction::InstallSilent
                    )
                }) {
                    view.install_update(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn install_update(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.update_status, UpdateStatus::Ready(..)) {
            return;
        }
        let UpdateStatus::Ready(version, installer, _) =
            mem::replace(&mut self.update_status, UpdateStatus::Installing)
        else {
            unreachable!();
        };
        cx.notify();
        let smoke = self.update_smoke.clone();
        let silent = smoke
            .as_ref()
            .is_some_and(|smoke| smoke.action == SmokeAction::InstallSilent);
        let install = cx.background_executor().spawn(async move {
            let result = (|| -> anyhow::Result<_> {
                let mut process = updates::launch(&installer, silent)?;
                if let Some(smoke) = &smoke {
                    smoke.record(&format!("installer:{}", process.id()));
                }
                let status = process
                    .wait()
                    .context("could not wait for OpenPlayer Setup")?;
                if let Some(smoke) = &smoke {
                    smoke.record(&format!("installer-exit:{}", status.code().unwrap_or(-1)));
                }
                Ok(status)
            })();
            (installer, result)
        });
        cx.spawn(async move |view, cx| {
            let (installer, result) = install.await;
            let _ = view.update(cx, |view, cx| {
                let error = match result {
                    // NSIS returns 1 when a user cancels at the confirmation page.
                    Ok(status) if status.success() || status.code() == Some(1) => None,
                    Ok(status) => Some(format!("OpenPlayer Setup exited with {status}")),
                    Err(error) => {
                        if let Some(smoke) = &view.update_smoke {
                            smoke.record(&format!("error:{error:#}"));
                        }
                        Some(format!("{error:#}"))
                    }
                };
                view.update_status = UpdateStatus::Ready(version, installer, error);
                cx.notify();
            });
        })
        .detach();
    }
}

impl Drop for PlayerView {
    fn drop(&mut self) {
        if let UpdateStatus::Ready(_, installer, _) = mem::take(&mut self.update_status)
            && let Err(error) = installer.remove()
        {
            eprintln!("OpenPlayer update cleanup: {error:#}");
        }
    }
}
