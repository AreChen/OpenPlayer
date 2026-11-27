use std::{
    ffi::c_void,
    io,
    ptr::{null, null_mut},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use windows_sys::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE, WINHTTP_QUERY_FLAG_NUMBER,
    WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};

const RELEASE_PATH: &str = "/repos/AreChen/OpenPlayer/releases";
const RELEASE_DOWNLOADS: &str = "https://github.com/AreChen/OpenPlayer/releases/download";
const PAGE_SIZE: usize = 100;
const RESPONSE_LIMIT: usize = 16 * 1024 * 1024;
const REQUEST_TIMEOUT_MS: i32 = 8_000;
const CHECK_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Default)]
pub(super) enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Current,
    Available(ReleaseUpdate),
    Failed(String),
}

pub(super) struct ReleaseUpdate {
    pub(super) version: String,
    pub(super) url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    state: String,
}

type Version = [u32; 3];

pub(super) fn check() -> Result<Option<ReleaseUpdate>> {
    let current = stable_version(env!("CARGO_PKG_VERSION")).context("unsupported app version")?;
    let http = GitHub::new()?;
    let deadline = Instant::now() + CHECK_TIMEOUT;
    let mut newest = None;
    for page in 1.. {
        ensure!(Instant::now() < deadline, "GitHub release check timed out");
        let releases = http.releases(page, deadline)?;
        let last_page = releases.len() < PAGE_SIZE;
        if let Some(candidate) = latest_update(releases, current)
            && newest
                .as_ref()
                .is_none_or(|(version, _)| candidate.0 > *version)
        {
            newest = Some(candidate);
        }
        if last_page {
            break;
        }
    }
    Ok(newest.map(|(_, update)| update))
}

fn latest_update(
    releases: impl IntoIterator<Item = Release>,
    current: Version,
) -> Option<(Version, ReleaseUpdate)> {
    releases
        .into_iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            let version = stable_version(&release.tag_name)?;
            if version[0] != current[0] || version <= current {
                return None;
            }
            let [major, minor, patch] = version;
            let version_text = format!("{major}.{minor}.{patch}");
            let arch = match std::env::consts::ARCH {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                _ => return None,
            };
            let installer = format!("OpenPlayer_{version_text}_{arch}-setup.exe");
            let has_installer = release
                .assets
                .iter()
                .any(|asset| asset.name == installer && asset.state == "uploaded");
            has_installer.then(|| {
                let url = format!("{RELEASE_DOWNLOADS}/{}/{installer}", release.tag_name);
                (
                    version,
                    ReleaseUpdate {
                        version: version_text,
                        url,
                    },
                )
            })
        })
        .max_by_key(|(version, _)| *version)
}

fn stable_version(text: &str) -> Option<Version> {
    let mut parts = text.strip_prefix('v').unwrap_or(text).split('.');
    let mut next = || {
        let part = parts.next()?;
        if !part.bytes().all(|byte| byte.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return None;
        }
        part.parse::<u32>().ok()
    };
    let version = [next()?, next()?, next()?];
    parts.next().is_none().then_some(version)
}

struct GitHub {
    connection: HttpHandle,
    _session: HttpHandle,
}

impl GitHub {
    fn new() -> Result<Self> {
        let agent = wide(&format!("OpenPlayer/{}", env!("CARGO_PKG_VERSION")));
        let host = wide("api.github.com");
        // All WinHTTP operations run on GPUI's background executor. WinHTTP owns TLS and proxy settings.
        let session = HttpHandle::new(unsafe {
            WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                null(),
                null(),
                0,
            )
        })?;
        let timeout = REQUEST_TIMEOUT_MS;
        checked(unsafe { WinHttpSetTimeouts(session.0, timeout, timeout, timeout, timeout) })?;
        let connection =
            HttpHandle::new(unsafe { WinHttpConnect(session.0, host.as_ptr(), 443, 0) })?;
        Ok(Self {
            connection,
            _session: session,
        })
    }

    fn releases(&self, page: usize, deadline: Instant) -> Result<Vec<Release>> {
        let path = wide(&format!("{RELEASE_PATH}?per_page={PAGE_SIZE}&page={page}"));
        let headers = wide("Accept: application/vnd.github+json\r\n");
        let request = HttpHandle::new(unsafe {
            WinHttpOpenRequest(
                self.connection.0,
                null(),
                path.as_ptr(),
                null(),
                null(),
                null(),
                WINHTTP_FLAG_SECURE,
            )
        })?;
        checked(unsafe {
            WinHttpSendRequest(request.0, headers.as_ptr(), u32::MAX, null(), 0, 0, 0)
        })?;
        checked(unsafe { WinHttpReceiveResponse(request.0, null_mut()) })?;
        let mut status: u32 = 0;
        let mut len = size_of::<u32>() as u32;
        checked(unsafe {
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                null(),
                (&raw mut status).cast(),
                &mut len,
                null_mut(),
            )
        })?;
        ensure!(status == 200, "GitHub HTTP {status}");

        let mut body = Vec::new();
        let mut buffer = [0u8; 8 * 1024];
        loop {
            ensure!(Instant::now() < deadline, "GitHub release check timed out");
            let mut read = 0;
            checked(unsafe {
                WinHttpReadData(
                    request.0,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut read,
                )
            })?;
            if read == 0 {
                break;
            }
            ensure!(
                body.len() + read as usize <= RESPONSE_LIMIT,
                "GitHub response is too large"
            );
            body.extend_from_slice(&buffer[..read as usize]);
        }
        serde_json::from_slice(&body).context("invalid GitHub release list")
    }
}

struct HttpHandle(*mut c_void);

impl HttpHandle {
    fn new(handle: *mut c_void) -> io::Result<Self> {
        if handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }
}

impl Drop for HttpHandle {
    fn drop(&mut self) {
        unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn checked(result: i32) -> io::Result<()> {
    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

#[cfg(test)]
mod tests;
