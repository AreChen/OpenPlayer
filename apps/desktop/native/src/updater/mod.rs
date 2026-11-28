use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use reqwest::{StatusCode, Url, blocking::Client, redirect::Policy};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const RELEASE_PATH: &str = "repos/AreChen/OpenPlayer/releases";
const DOWNLOAD_PATH: &str = "AreChen/OpenPlayer/releases/download";
const PAGE_SIZE: usize = 100;
const RELEASE_LIST_LIMIT: usize = 16 * 1024 * 1024;
const CHECKSUM_LIMIT: u64 = 1024;
const PACKAGE_LIMIT: u64 = 2 * 1024 * 1024 * 1024;
const CHECK_TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Clone, Copy, Debug)]
pub(crate) enum Target {
    #[cfg(any(windows, test))]
    WindowsNsis,
    #[cfg(any(target_os = "macos", test))]
    MacDmg,
    #[cfg(any(target_os = "linux", test))]
    LinuxDeb,
    #[cfg(any(target_os = "linux", test))]
    LinuxAppImage,
}

impl Target {
    fn asset_name(self, version: &str, arch: &str) -> Option<String> {
        let suffix = match (self, arch) {
            #[cfg(any(windows, test))]
            (Self::WindowsNsis, "x86_64") => "x64-setup.exe",
            #[cfg(any(target_os = "macos", test))]
            (Self::MacDmg, "aarch64") => "arm64.dmg",
            #[cfg(any(target_os = "macos", test))]
            (Self::MacDmg, "x86_64") => "x64.dmg",
            #[cfg(any(target_os = "linux", test))]
            (Self::LinuxDeb, "x86_64") => "amd64.deb",
            #[cfg(any(target_os = "linux", test))]
            (Self::LinuxAppImage, "x86_64") => "amd64.AppImage",
            _ => return None,
        };
        Some(format!("OpenPlayer_{version}_{suffix}"))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Update {
    pub(crate) version: String,
    tag: String,
    name: String,
    size: u64,
    checksum_size: u64,
    source: Source,
}

#[derive(Debug)]
pub(crate) struct DownloadedUpdate {
    path: PathBuf,
    size: u64,
    sha256: [u8; 32],
}

impl DownloadedUpdate {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn verify(&self, path: &Path) -> Result<()> {
        let mut file = File::open(path).context("open staged update package")?;
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file() && metadata.len() == self.size,
            "update package size does not match the release"
        );
        verify_body(&mut file, &mut io::sink(), self.size, self.sha256)
    }

    pub(crate) fn remove(self) -> Result<()> {
        fs::remove_file(&self.path).context("remove staged update package")?;
        if let Some(directory) = self.path.parent() {
            fs::remove_dir(directory).context("remove update staging directory")?;
        }
        Ok(())
    }
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
    size: u64,
}

impl Release {
    fn asset_size(&self, name: &str, limit: u64) -> Option<u64> {
        self.assets
            .iter()
            .find(|asset| asset.name == name && asset.state == "uploaded")
            .map(|asset| asset.size)
            .filter(|size| (1..=limit).contains(size))
    }
}

type Version = [u32; 3];

pub(crate) fn check(target: Target) -> Result<Option<Update>> {
    let current = stable_version(env!("CARGO_PKG_VERSION")).context("unsupported app version")?;
    check_from(
        current,
        target,
        std::env::consts::ARCH,
        Source::configured()?,
    )
}

pub(crate) fn test_updates_enabled() -> bool {
    matches!(Source::configured(), Ok(Source::Local(_)))
}

fn check_from(
    current: Version,
    target: Target,
    arch: &str,
    source: Source,
) -> Result<Option<Update>> {
    if current[0] != 2 || target.asset_name("", arch).is_none() {
        return Ok(None);
    }
    let client = source.client()?;
    let deadline = Instant::now() + CHECK_TIMEOUT;
    let mut newest = None;
    for page in 1.. {
        let remaining = deadline.saturating_duration_since(Instant::now());
        ensure!(!remaining.is_zero(), "update check timed out");
        let mut url = source.url("api.github.com", RELEASE_PATH)?;
        url.query_pairs_mut()
            .append_pair("per_page", &PAGE_SIZE.to_string())
            .append_pair("page", &page.to_string());
        let response = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .timeout(remaining)
            .send()
            .context("fetch update release list")?;
        let body = small_body(response, RELEASE_LIST_LIMIT)?;
        let releases: Vec<Release> =
            serde_json::from_slice(&body).context("invalid update release list")?;
        let last_page = releases.len() < PAGE_SIZE;
        if let Some(candidate) = latest_update(releases, current, target, arch, &source)
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
    target: Target,
    arch: &str,
    source: &Source,
) -> Option<(Version, Update)> {
    releases
        .into_iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            let version = stable_version(release.tag_name.strip_prefix('v')?)?;
            if version[0] != current[0] || version <= current {
                return None;
            }
            let [major, minor, patch] = version;
            let version_text = format!("{major}.{minor}.{patch}");
            let name = target.asset_name(&version_text, arch)?;
            let size = release.asset_size(&name, PACKAGE_LIMIT)?;
            let checksum_size = release.asset_size(&format!("{name}.sha256"), CHECKSUM_LIMIT)?;
            Some((
                version,
                Update {
                    version: version_text,
                    tag: release.tag_name,
                    name,
                    size,
                    checksum_size,
                    source: source.clone(),
                },
            ))
        })
        .max_by_key(|(version, _)| *version)
}

fn stable_version(text: &str) -> Option<Version> {
    let mut parts = text.split('.');
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

pub(crate) fn download(update: &Update, staging_directory: &Path) -> Result<DownloadedUpdate> {
    let client = update.source.client()?;
    let asset = format!("{DOWNLOAD_PATH}/{}/{}", update.tag, update.name);
    let checksum_url = update
        .source
        .url("github.com", &format!("{asset}.sha256"))?;
    let checksum_body = small_body(
        client
            .get(checksum_url)
            .timeout(CHECK_TIMEOUT)
            .send()
            .context("fetch update checksum")?,
        CHECKSUM_LIMIT as usize,
    )?;
    ensure!(
        checksum_body.len() as u64 == update.checksum_size,
        "update checksum size does not match the release"
    );
    let sha256 = parse_checksum(&checksum_body, &update.name)?;

    fs::create_dir_all(staging_directory).context("create update cache")?;
    let directory = tempfile::Builder::new()
        .prefix("download-")
        .tempdir_in(staging_directory)
        .context("create update staging directory")?;
    let path = directory.path().join(&update.name);
    let mut file = File::create_new(path).context("create staged update package")?;
    let url = update.source.url("github.com", &asset)?;
    let mut response = client
        .get(url)
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .context("download update package")?;
    require_ok(&response)?;
    ensure!(
        response
            .content_length()
            .is_none_or(|size| size == update.size),
        "update package size does not match the release"
    );
    verify_body(&mut response, &mut file, update.size, sha256)?;
    file.sync_all().context("sync staged update package")?;
    drop(file);

    Ok(DownloadedUpdate {
        path: directory.keep().join(&update.name),
        size: update.size,
        sha256,
    })
}

fn parse_checksum(bytes: &[u8], name: &str) -> Result<[u8; 32]> {
    let line = std::str::from_utf8(bytes).context("invalid update checksum text")?;
    let mut fields = line.trim_end_matches(['\r', '\n']).split_ascii_whitespace();
    let digest = fields.next().context("missing update checksum")?;
    let file = fields.next().context("missing update checksum filename")?;
    ensure!(
        file.strip_prefix('*').unwrap_or(file) == name
            && fields.next().is_none()
            && line.lines().count() == 1,
        "update checksum names a different package"
    );
    let mut sha256 = [0; 32];
    hex::decode_to_slice(digest, &mut sha256).context("invalid update SHA-256")?;
    Ok(sha256)
}

fn verify_body(
    input: &mut impl Read,
    output: &mut impl Write,
    size: u64,
    sha256: [u8; 32],
) -> Result<()> {
    let mut bounded = input.take(size + 1);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 32 * 1024];
    let mut total = 0;
    loop {
        let count = bounded.read(&mut buffer).context("read update package")?;
        if count == 0 {
            break;
        }
        total += count as u64;
        ensure!(total <= size, "update package exceeds the declared size");
        hasher.update(&buffer[..count]);
        output
            .write_all(&buffer[..count])
            .context("write staged update package")?;
    }
    ensure!(total == size, "update package download is incomplete");
    ensure!(
        hasher.finalize().as_slice() == sha256,
        "update package SHA-256 does not match the release"
    );
    Ok(())
}

fn require_ok(response: &reqwest::blocking::Response) -> Result<()> {
    ensure!(
        response.status() == StatusCode::OK,
        "update server returned HTTP {}",
        response.status()
    );
    Ok(())
}

fn small_body(response: reqwest::blocking::Response, limit: usize) -> Result<Vec<u8>> {
    require_ok(&response)?;
    let mut body = Vec::new();
    response
        .take((limit + 1) as u64)
        .read_to_end(&mut body)
        .context("read update server response")?;
    ensure!(body.len() <= limit, "update server response is too large");
    Ok(body)
}

#[derive(Clone, Debug)]
enum Source {
    GitHub,
    Local(Url),
}

impl Source {
    fn configured() -> Result<Self> {
        if option_env!("OPENPLAYER_ENABLE_TEST_UPDATES") == Some("1")
            && let Some(origin) = std::env::var_os("OPENPLAYER_TEST_UPDATE_ORIGIN")
        {
            return Self::local(origin.to_str().context("invalid test update origin")?);
        }
        Ok(Self::GitHub)
    }

    fn local(origin: &str) -> Result<Self> {
        let origin = Url::parse(origin).context("invalid test update origin")?;
        ensure!(
            origin.scheme() == "http"
                && matches!(origin.host_str(), Some("127.0.0.1" | "[::1]"))
                && origin.port().is_some()
                && origin.path() == "/"
                && origin.query().is_none()
                && origin.fragment().is_none()
                && origin.username().is_empty()
                && origin.password().is_none(),
            "test update origin must be a literal loopback HTTP origin with a port"
        );
        Ok(Self::Local(origin))
    }

    fn url(&self, host: &str, path: &str) -> Result<Url> {
        match self {
            Self::GitHub => Ok(Url::parse(&format!("https://{host}/{path}"))?),
            Self::Local(origin) => Ok(origin.join(path)?),
        }
    }

    fn client(&self) -> Result<Client> {
        let source = self.clone();
        let builder = Client::builder()
            .user_agent(concat!("OpenPlayer/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(8))
            .referer(false)
            .redirect(Policy::custom(move |attempt| {
                if attempt.previous().len() < 5 && source.allows(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.error("unexpected update download redirect")
                }
            }));
        let builder = match self {
            Self::GitHub => builder.https_only(true),
            Self::Local(_) => builder.no_proxy(),
        };
        builder.build().context("initialize update HTTP client")
    }

    fn allows(&self, url: &Url) -> bool {
        match self {
            Self::GitHub => {
                url.scheme() == "https"
                    && url.port_or_known_default() == Some(443)
                    && matches!(
                        url.host_str(),
                        Some(
                            "api.github.com"
                                | "github.com"
                                | "objects.githubusercontent.com"
                                | "release-assets.githubusercontent.com"
                        )
                    )
                    && url.username().is_empty()
                    && url.password().is_none()
            }
            Self::Local(origin) => url.origin() == origin.origin(),
        }
    }
}

#[cfg(test)]
mod tests;
