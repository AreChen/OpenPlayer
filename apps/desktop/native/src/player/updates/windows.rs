use super::{InstallResult, SmokeAction, UpdateSmoke};
use crate::updater::{self, DownloadedUpdate, Target, Update};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    process::{Child, Command},
    ptr::null_mut,
};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
};
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPlayer";
const PLAYER_EXE: &str = "openplayer-desktop.exe";

pub(in crate::player) fn check() -> Result<Option<Update>> {
    installed_executable()?;
    updater::check(Target::WindowsNsis)
}

pub(in crate::player) fn download(update: &Update) -> Result<DownloadedUpdate> {
    let data = std::env::var_os("LOCALAPPDATA").context("Windows local app data is unavailable")?;
    updater::download(
        update,
        &PathBuf::from(data).join("OpenPlayer").join("updates"),
    )
}

fn launch(installer: &DownloadedUpdate, silent: bool) -> Result<Child> {
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
    let directory = PathBuf::from(OsString::from_wide(
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

pub(in crate::player) fn install(
    installer: &DownloadedUpdate,
    _: &str,
    smoke: Option<&UpdateSmoke>,
) -> Result<InstallResult> {
    let mut process = launch(
        installer,
        smoke.is_some_and(|s| s.action == SmokeAction::InstallSilent),
    )?;
    if let Some(s) = smoke {
        s.record(&format!("installer:{}", process.id()));
    }
    let status = process
        .wait()
        .context("could not wait for OpenPlayer Setup")?;
    if let Some(s) = smoke {
        s.record(&format!("installer-exit:{}", status.code().unwrap_or(-1)));
    }
    ensure!(
        status.success() || status.code() == Some(1),
        "OpenPlayer Setup exited with {status}"
    );
    Ok(InstallResult::Ready)
}
