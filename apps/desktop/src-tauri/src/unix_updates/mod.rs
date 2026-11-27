#[cfg(feature = "mpv-embed")]
mod fixture;
mod package;

use std::sync::Mutex;

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use tauri::{AppHandle, Manager, async_runtime::spawn_blocking};

use crate::updater::{self, DownloadedUpdate, Update};
#[cfg(feature = "mpv-embed")]
pub(crate) use fixture::start_fixture_test;
use package::{InstallOutcome, InstalledPackage, PackageKind};

#[derive(Default)]
pub(crate) struct DesktopUpdater(Mutex<UpdateStage>);

#[derive(Default)]
enum UpdateStage {
    #[default]
    Idle,
    Available(Offer),
    Ready(Offer, DownloadedUpdate),
    #[cfg(target_os = "linux")]
    Applied(UpdateOffer),
}

struct Offer {
    update: Update,
    installed_package: InstalledPackage,
}

#[derive(Clone, Serialize)]
pub(crate) struct UpdateOffer {
    version: String,
    package: PackageKind,
}

impl Offer {
    fn summary(&self) -> UpdateOffer {
        UpdateOffer {
            version: self.update.version.clone(),
            package: self.installed_package.kind(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub(crate) enum CheckResult {
    Current,
    Unsupported,
    Available {
        update: UpdateOffer,
    },
    #[cfg(target_os = "linux")]
    Installed {
        update: UpdateOffer,
    },
}

#[derive(Serialize)]
pub(crate) struct InstallResult {
    status: InstallOutcome,
}

impl DesktopUpdater {
    fn check(&self) -> Result<CheckResult> {
        let mut stage = self.0.try_lock().map_err(|_| busy())?;
        match &*stage {
            UpdateStage::Ready(offer, _) => {
                return Ok(CheckResult::Available {
                    update: offer.summary(),
                });
            }
            #[cfg(target_os = "linux")]
            UpdateStage::Applied(update) => {
                return Ok(CheckResult::Installed {
                    update: update.clone(),
                });
            }
            _ => {}
        }
        let Some(installed_package) = InstalledPackage::detect() else {
            return Ok(CheckResult::Unsupported);
        };
        let Some(update) = updater::check(installed_package.target())? else {
            *stage = UpdateStage::Idle;
            return Ok(CheckResult::Current);
        };
        let offer = Offer {
            update,
            installed_package,
        };
        let update = offer.summary();
        *stage = UpdateStage::Available(offer);
        Ok(CheckResult::Available { update })
    }

    fn download(&self, app: &AppHandle, version: &str) -> Result<()> {
        let mut stage = self.0.try_lock().map_err(|_| busy())?;
        if let UpdateStage::Ready(offer, _) = &*stage {
            return ensure_version(&offer.update, version);
        }
        let UpdateStage::Available(offer) = &*stage else {
            bail!("no update package is available");
        };
        ensure_version(&offer.update, version)?;
        let directory = app.path().app_cache_dir()?.join("updates");
        let download = updater::download(&offer.update, &directory)?;
        if let UpdateStage::Available(offer) = std::mem::take(&mut *stage) {
            *stage = UpdateStage::Ready(offer, download);
        }
        Ok(())
    }

    fn install(&self, version: &str) -> Result<InstallResult> {
        let mut stage = self.0.try_lock().map_err(|_| busy())?;
        let UpdateStage::Ready(offer, _) = &*stage else {
            bail!("no verified update package is ready to install");
        };
        ensure_version(&offer.update, version)?;
        let UpdateStage::Ready(offer, package) = std::mem::take(&mut *stage) else {
            unreachable!("update state is locked");
        };

        let result = offer.installed_package.install(&package, version);
        *stage = match &result {
            #[cfg(target_os = "macos")]
            Ok(InstallOutcome::InstallerOpened) => UpdateStage::Ready(offer, package),
            #[cfg(target_os = "linux")]
            Ok(InstallOutcome::Installed) => {
                remove_download(package);
                UpdateStage::Applied(offer.summary())
            }
            Err(_) => {
                remove_download(package);
                UpdateStage::Available(offer)
            }
        };
        result.map(|status| InstallResult { status })
    }
}

fn remove_download(package: DownloadedUpdate) {
    if let Err(error) = package.remove() {
        eprintln!("could not remove the staged update package: {error:#}");
    }
}

fn busy() -> anyhow::Error {
    anyhow::anyhow!("another update operation is in progress")
}

fn ensure_version(update: &Update, version: &str) -> Result<()> {
    ensure!(
        update.version == version,
        "the available update has changed"
    );
    Ok(())
}

#[tauri::command]
pub(crate) async fn app_update_check(app: AppHandle) -> Result<CheckResult, String> {
    run_update_task(move || app.state::<DesktopUpdater>().check()).await
}

#[tauri::command]
pub(crate) async fn app_update_download(app: AppHandle, version: String) -> Result<(), String> {
    run_update_task(move || app.state::<DesktopUpdater>().download(&app, &version)).await
}

#[tauri::command]
pub(crate) async fn app_update_install(
    app: AppHandle,
    version: String,
) -> Result<InstallResult, String> {
    run_update_task(move || app.state::<DesktopUpdater>().install(&version)).await
}

#[tauri::command]
pub(crate) async fn app_update_restart(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let state = app.state::<DesktopUpdater>();
        let stage = state.0.try_lock().map_err(|error| error.to_string())?;
        if !matches!(*stage, UpdateStage::Applied(_)) {
            return Err("an installed update is required before restarting".into());
        }
        drop(stage);

        #[cfg(feature = "mpv-embed")]
        crate::mpv_embed::stop_embedded_player_for_close(&app)?;
        app.request_restart();
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Err("use Finder to replace and reopen OpenPlayer".into())
    }
}

async fn run_update_task<T: Send + 'static>(
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T, String> {
    spawn_blocking(work)
        .await
        .context("update task did not complete")
        .and_then(|result| result)
        .map_err(|error| format!("{error:#}"))
}
