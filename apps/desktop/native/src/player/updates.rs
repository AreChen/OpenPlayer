use crate::updater::{self, DownloadedUpdate, Update};
use anyhow::{Result, ensure};
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::{check, download, install};
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub(super) use unix::{check, download, install};

pub(super) enum InstallResult {
    #[cfg(windows)]
    Ready,
    #[cfg(target_os = "macos")]
    Opened,
    #[cfg(target_os = "linux")]
    Installed(PathBuf),
}

#[derive(Default)]
pub(super) enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Current,
    Available(Update, Option<String>),
    Downloading,
    Ready(String, DownloadedUpdate, Option<String>),
    Installing,
    #[cfg(target_os = "linux")]
    Installed(PathBuf),
    #[cfg(target_os = "macos")]
    Opened(DownloadedUpdate),
    Failed(String),
}

impl UpdateStatus {
    pub(super) fn restart_path(&self) -> Option<&std::path::Path> {
        #[cfg(target_os = "linux")]
        if let Self::Installed(path) = self {
            return Some(path);
        }
        None
    }

    pub(super) fn can_check(&self) -> bool {
        matches!(
            self,
            Self::Idle | Self::Current | Self::Available(..) | Self::Failed(_)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SmokeAction {
    Verify,
    Install,
    InstallSilent,
    Playback,
}

#[derive(Clone)]
pub(super) struct UpdateSmoke {
    pub(super) action: SmokeAction,
    pub(super) wait_for_playback: bool,
    status: PathBuf,
}

impl UpdateSmoke {
    pub(super) fn new(action: Option<&str>, status: PathBuf) -> Result<Self> {
        ensure!(
            updater::test_updates_enabled()
                && (std::env::var_os("OPENPLAYER_WINDOWS_SMOKE_PROFILE").is_some()
                    || std::env::var_os("OPENPLAYER_SMOKE_PROFILE").is_some()),
            "the update smoke harness requires a fixture build, loopback source and smoke profile"
        );
        let action = match action {
            Some("verify") => SmokeAction::Verify,
            Some("install") => SmokeAction::Install,
            Some("install-silent") => SmokeAction::InstallSilent,
            Some("playback") => SmokeAction::Playback,
            _ => anyhow::bail!("invalid smoke update action"),
        };
        Ok(Self {
            action,
            wait_for_playback: false,
            status,
        })
    }

    pub(super) fn record(&self, event: &str) {
        let result = (|| -> io::Result<()> {
            let mut log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.status)?;
            writeln!(log, "{}", event.replace(['\r', '\n'], " "))
        })();
        if let Err(error) = result {
            eprintln!("OpenPlayer update smoke status: {error}");
        }
    }
}
