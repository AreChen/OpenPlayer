use std::process::Command;

#[cfg(target_os = "linux")]
use std::{
    fs::{self, File},
    io::{self, Read},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Stdio,
};

#[cfg(target_os = "linux")]
use anyhow::ensure;
use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::updater::{DownloadedUpdate, Target};

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum PackageKind {
    #[cfg(target_os = "macos")]
    Dmg,
    #[cfg(target_os = "linux")]
    Deb,
    #[cfg(target_os = "linux")]
    AppImage,
}

pub(super) enum InstalledPackage {
    #[cfg(target_os = "macos")]
    Dmg,
    #[cfg(target_os = "linux")]
    Deb(String),
    #[cfg(target_os = "linux")]
    AppImage {
        path: PathBuf,
        device: u64,
        inode: u64,
    },
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum InstallOutcome {
    #[cfg(target_os = "linux")]
    Installed,
    #[cfg(target_os = "macos")]
    InstallerOpened,
}

impl InstalledPackage {
    pub(super) fn kind(&self) -> PackageKind {
        match self {
            #[cfg(target_os = "macos")]
            Self::Dmg => PackageKind::Dmg,
            #[cfg(target_os = "linux")]
            Self::Deb(_) => PackageKind::Deb,
            #[cfg(target_os = "linux")]
            Self::AppImage { .. } => PackageKind::AppImage,
        }
    }

    pub(super) fn target(&self) -> Target {
        match self {
            #[cfg(target_os = "macos")]
            Self::Dmg => Target::MacDmg,
            #[cfg(target_os = "linux")]
            Self::Deb(_) => Target::LinuxDeb,
            #[cfg(target_os = "linux")]
            Self::AppImage { .. } => Target::LinuxAppImage,
        }
    }

    pub(super) fn detect() -> Option<Self> {
        let executable = std::env::current_exe().ok()?;
        #[cfg(target_os = "macos")]
        {
            let macos = executable.parent()?;
            let contents = macos.parent()?;
            let bundle = contents.parent()?;
            (macos.ends_with("MacOS")
                && contents.ends_with("Contents")
                && bundle.extension().is_some_and(|ext| ext == "app")
                && contents.join("Info.plist").is_file())
            .then_some(Self::Dmg)
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(appimage) = std::env::var_os("APPIMAGE") {
                let image = PathBuf::from(appimage).canonicalize().ok()?;
                let appdir = PathBuf::from(std::env::var_os("APPDIR")?)
                    .canonicalize()
                    .ok()?;
                let metadata = fs::metadata(&image).ok()?;
                return (metadata.is_file()
                    && executable.starts_with(appdir)
                    && is_appimage(&image))
                .then_some(Self::AppImage {
                    path: image,
                    device: metadata.dev(),
                    inode: metadata.ino(),
                });
            }

            let ownership = Command::new("/usr/bin/dpkg-query")
                .arg("--search")
                .arg(&executable)
                .output()
                .ok()?;
            if !ownership.status.success() {
                return None;
            }
            let listing = String::from_utf8(ownership.stdout).ok()?;
            let name = deb_owner(&listing, executable.to_str()?)?;
            let status = Command::new("/usr/bin/dpkg-query")
                .args([
                    "--show",
                    "--showformat=${db:Status-Status}\n${Version}",
                    name,
                ])
                .output()
                .ok()?;
            (status.status.success()
                && status.stdout == format!("installed\n{}", env!("CARGO_PKG_VERSION")).as_bytes())
            .then(|| Self::Deb(name.to_owned()))
        }
    }

    pub(super) fn install(
        &self,
        update: &DownloadedUpdate,
        version: &str,
    ) -> Result<InstallOutcome> {
        update.verify(update.path())?;
        match self {
            #[cfg(target_os = "macos")]
            Self::Dmg => {
                let _ = version;
                let output = Command::new("/usr/bin/open")
                    .arg(update.path())
                    .output()
                    .context("could not open the verified disk image")?;
                require_success(output, "opening the verified disk image")?;
                Ok(InstallOutcome::InstallerOpened)
            }
            #[cfg(target_os = "linux")]
            Self::Deb(name) => {
                install_deb(name, version, update.path())?;
                Ok(InstallOutcome::Installed)
            }
            #[cfg(target_os = "linux")]
            Self::AppImage {
                path,
                device,
                inode,
            } => {
                replace_appimage(path, (*device, *inode), update)?;
                Ok(InstallOutcome::Installed)
            }
        }
    }
}

fn require_success(output: std::process::Output, action: &str) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let details = stderr.trim();
    if details.is_empty() {
        bail!("{action} failed ({})", output.status);
    }
    let details: String = details.chars().take(600).collect();
    bail!("{action} failed ({}): {details}", output.status)
}

#[cfg(target_os = "linux")]
fn deb_owner<'a>(listing: &'a str, executable: &str) -> Option<&'a str> {
    listing.lines().find_map(|line| {
        let (owner, path) = line.split_once(": ")?;
        let package = owner.split(':').next()?;
        (path == executable && matches!(package, "open-player" | "openplayer")).then_some(package)
    })
}

#[cfg(target_os = "linux")]
fn install_deb(name: &str, version: &str, package: &Path) -> Result<()> {
    let metadata = Command::new("/usr/bin/dpkg-deb")
        .arg("--field")
        .arg(package)
        .args(["Package", "Version", "Architecture"])
        .output()
        .context("could not read Debian package metadata")?;
    ensure!(
        metadata.status.success(),
        "could not read Debian package metadata"
    );
    let metadata = String::from_utf8(metadata.stdout).context("invalid Debian package metadata")?;
    let architecture = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => bail!("unsupported Debian package architecture"),
    };
    let field = |name| {
        metadata
            .lines()
            .filter_map(|line| line.split_once(": "))
            .find_map(|(key, value)| (key == name).then_some(value))
    };
    ensure!(
        field("Package") == Some(name)
            && field("Version") == Some(version)
            && field("Architecture") == Some(architecture),
        "the downloaded Debian package does not match the installed application"
    );
    let output = Command::new("/usr/bin/pkexec")
        .args([
            "--disable-internal-agent",
            "/usr/bin/apt-get",
            "--yes",
            "--no-remove",
            "install",
        ])
        .arg(package)
        .stdin(Stdio::null())
        .output()
        .context("could not start the system package installer")?;
    if output.status.code() == Some(126) {
        bail!("authorization for the package installation was cancelled");
    }
    require_success(output, "system package installation")
}

#[cfg(target_os = "linux")]
fn is_appimage(path: &Path) -> bool {
    let mut header = [0; 11];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .is_ok()
        && &header[..4] == b"\x7fELF"
        && &header[8..] == b"AI\x02"
}

#[cfg(target_os = "linux")]
fn replace_appimage(image: &Path, identity: (u64, u64), update: &DownloadedUpdate) -> Result<()> {
    ensure!(
        is_appimage(image),
        "the running AppImage is no longer available"
    );
    ensure!(
        is_appimage(update.path()),
        "the downloaded file is not an AppImage"
    );
    let directory = image
        .parent()
        .context("the AppImage has no containing directory")?;
    // Stage on the same filesystem so the installed image remains usable until the rename succeeds.
    let mut replacement = tempfile::Builder::new()
        .prefix(".openplayer-update-")
        .tempfile_in(directory)
        .context("could not write an update in the AppImage directory")?;
    // Hold the old inode's lock through the rename to serialize simultaneous upgrades.
    let original = File::open(image)?;
    original
        .try_lock()
        .context("the AppImage is being updated")?;
    let metadata = original.metadata()?;
    ensure!(
        (metadata.dev(), metadata.ino()) == identity,
        "the AppImage has changed since checking for updates"
    );
    let permissions = metadata.permissions();
    io::copy(&mut File::open(update.path())?, replacement.as_file_mut())?;
    replacement
        .as_file()
        .set_permissions(fs::Permissions::from_mode(permissions.mode() & 0o777))?;
    replacement.as_file().sync_all()?;
    update.verify(replacement.path())?;
    let current = fs::metadata(image)?;
    ensure!(
        (current.dev(), current.ino()) == identity,
        "the AppImage has changed since checking for updates"
    );
    replacement
        .persist(image)
        .context("could not replace the AppImage")?;
    // Some filesystems cannot fsync a directory. The verified replacement is already installed.
    if let Err(error) = File::open(directory).and_then(|dir| dir.sync_all()) {
        eprintln!("could not sync the AppImage directory: {error}");
    }
    Ok(())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn identifies_only_openplayer_deb_ownership() {
        let path = "/usr/bin/openplayer-desktop";
        assert_eq!(
            deb_owner("open-player: /usr/bin/openplayer-desktop", path),
            Some("open-player")
        );
        assert_eq!(
            deb_owner("openplayer:amd64: /usr/bin/openplayer-desktop", path),
            Some("openplayer")
        );
        assert_eq!(
            deb_owner("other-player: /usr/bin/openplayer-desktop", path),
            None
        );
        assert_eq!(deb_owner("openplayer: /tmp/openplayer-desktop", path), None);
    }
}
