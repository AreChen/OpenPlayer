use super::{InstallResult, UpdateSmoke};
use crate::{
    unix_package::{InstallOutcome, InstalledPackage},
    updater::{self, DownloadedUpdate, Update},
};
use anyhow::{Context, Result};
use std::sync::OnceLock;

fn package() -> Result<&'static InstalledPackage> {
    static PACKAGE: OnceLock<Option<InstalledPackage>> = OnceLock::new();
    PACKAGE
        .get_or_init(InstalledPackage::detect)
        .as_ref()
        .context("updates require an installed OpenPlayer package")
}

pub(in crate::player) fn check() -> Result<Option<Update>> {
    updater::check(package()?.target())
}

pub(in crate::player) fn download(update: &Update) -> Result<DownloadedUpdate> {
    let directory = dirs::cache_dir()
        .context("cannot locate user cache")?
        .join("OpenPlayer/updates");
    updater::download(update, &directory)
}

pub(in crate::player) fn install(
    file: &DownloadedUpdate,
    version: &str,
    smoke: Option<&UpdateSmoke>,
) -> Result<InstallResult> {
    let installed = package()?;
    #[cfg(target_os = "linux")]
    let restart_path = match installed {
        InstalledPackage::AppImage { path, .. } => path.clone(),
        InstalledPackage::Deb(_) => std::env::current_exe()?,
    };
    match installed.install(file, version)? {
        #[cfg(target_os = "linux")]
        InstallOutcome::Installed => {
            if let Some(s) = smoke {
                s.record("installed");
            }
            Ok(InstallResult::Installed(restart_path))
        }
        #[cfg(target_os = "macos")]
        InstallOutcome::InstallerOpened => {
            if let Some(s) = smoke {
                s.record(&format!("opened:{}", file.path().display()));
            }
            Ok(InstallResult::Opened)
        }
    }
}
