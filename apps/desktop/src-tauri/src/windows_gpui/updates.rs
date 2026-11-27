use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    process::{Child, Command},
    ptr::null_mut,
};

use anyhow::{Context, Result, ensure};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
};

use crate::updater::{self, DownloadedUpdate, Target, Update};

const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPlayer";
const PLAYER_EXE: &str = "openplayer-desktop.exe";

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
    Failed(String),
}

impl UpdateStatus {
    pub(super) fn can_check(&self) -> bool {
        matches!(
            self,
            Self::Idle | Self::Current | Self::Available(..) | Self::Failed(_)
        )
    }
}

pub(super) fn check() -> Result<Option<Update>> {
    installed_executable()?;
    updater::check(Target::WindowsNsis)
}

pub(super) fn download(update: &Update) -> Result<DownloadedUpdate> {
    let data = std::env::var_os("LOCALAPPDATA").context("Windows local app data is unavailable")?;
    updater::download(
        update,
        &PathBuf::from(data).join("OpenPlayer").join("updates"),
    )
}

pub(super) fn launch(installer: &DownloadedUpdate, silent: bool) -> Result<Child> {
    installed_executable()?;
    let path = installer.path();
    installer.verify(path)?;
    let mut command = Command::new(path);
    command.current_dir(path.parent().context("missing installer directory")?);
    // The NSIS update mode reads the registered install directory and confirms the upgrade
    // before it closes the player. This installation is per user, so no elevation is needed.
    command.arg("/UPDATE");
    if silent {
        command.arg("/S");
    }
    command.spawn().context("could not start OpenPlayer Setup")
}

fn installed_executable() -> Result<()> {
    let key = wide(UNINSTALL_KEY);
    let field = wide("InstallLocation");
    let mut value = vec![0u16; 32_768];
    let mut size = (value.len() * size_of::<u16>()) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            field.as_ptr(),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            null_mut(),
            value.as_mut_ptr().cast(),
            &mut size,
        )
    };
    ensure!(
        result == 0,
        "OpenPlayer is not registered as an installed application"
    );
    let value = &value[..size as usize / size_of::<u16>()];
    let directory = PathBuf::from(std::ffi::OsString::from_wide(
        value.strip_suffix(&[0]).unwrap_or(value),
    ));
    ensure!(
        directory.is_absolute(),
        "OpenPlayer has an invalid installation path"
    );
    // A development or portable copy must not replace a separate registered installation.
    let installed = directory.join(PLAYER_EXE).canonicalize()?;
    let current = std::env::current_exe()?.canonicalize()?;
    ensure!(
        current == installed,
        "the running copy of OpenPlayer is not registered for updates"
    );
    Ok(())
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
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
                && std::env::var_os("OPENPLAYER_WINDOWS_SMOKE_PROFILE").is_some(),
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
