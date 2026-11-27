use super::*;
use serde_json::json;
use std::{
    io::{BufRead, BufReader},
    net::TcpListener,
    thread,
};

const INSTALLER: &[u8] = b"verified package";
const INSTALLER_NAME: &str = "OpenPlayer_2.0.1_amd64.AppImage";

fn asset(name: &str, size: usize) -> serde_json::Value {
    json!({ "name": name, "state": "uploaded", "size": size })
}

fn release(
    tag: &str,
    draft: bool,
    prerelease: bool,
    assets: Vec<serde_json::Value>,
) -> serde_json::Value {
    json!({ "tag_name": tag, "draft": draft, "prerelease": prerelease, "assets": assets })
}

fn names(version: &str, target: Target, arch: &str) -> Vec<serde_json::Value> {
    let name = target.asset_name(version, arch).unwrap();
    vec![asset(&name, 100), asset(&format!("{name}.sha256"), 100)]
}

#[test]
fn chooses_the_newest_complete_stable_package_for_the_target() {
    let target = Target::LinuxDeb;
    let arch = "x86_64";
    let releases = [
        release("v3.0.0", false, false, names("3.0.0", target, arch)),
        release("v2.0.9", true, false, names("2.0.9", target, arch)),
        release("v2.0.8", false, true, names("2.0.8", target, arch)),
        release("v2.0.07", false, false, names("2.0.7", target, arch)),
        release("v2.0.7-rc.1", false, false, names("2.0.7", target, arch)),
        release(
            "v2.0.6",
            false,
            false,
            names("2.0.6", Target::LinuxAppImage, arch),
        ),
        release(
            "v2.0.5",
            false,
            false,
            vec![asset("OpenPlayer_2.0.5_amd64.deb", 100)],
        ),
        release("v2.0.1", false, false, names("2.0.1", target, arch)),
        release("v2.0.4", false, false, names("2.0.4", target, arch)),
        release("v2.0.0", false, false, names("2.0.0", target, arch)),
        release("v1.6.10", false, false, names("1.6.10", target, arch)),
    ]
    .into_iter()
    .map(|value| serde_json::from_value::<Release>(value).unwrap());

    let (_, update) = latest_update(releases, [2, 0, 0], target, arch, &Source::GitHub).unwrap();
    assert_eq!(update.version, "2.0.4");
    assert_eq!(update.name, "OpenPlayer_2.0.4_amd64.deb");
}

#[test]
fn matches_each_package_and_architecture_by_exact_name() {
    let packages = [
        (Target::WindowsNsis, "x86_64", "x64-setup.exe"),
        (Target::MacDmg, "aarch64", "arm64.dmg"),
        (Target::MacDmg, "x86_64", "x64.dmg"),
        (Target::LinuxDeb, "x86_64", "amd64.deb"),
        (Target::LinuxAppImage, "x86_64", "amd64.AppImage"),
    ];
    for (target, arch, suffix) in packages {
        assert_eq!(
            target.asset_name("2.0.1", arch).unwrap(),
            format!("OpenPlayer_2.0.1_{suffix}")
        );
    }
    assert!(Target::WindowsNsis.asset_name("2.0.1", "aarch64").is_none());
    assert!(Target::LinuxDeb.asset_name("2.0.1", "aarch64").is_none());
}

#[test]
fn restricts_redirects_and_test_origins() {
    assert!(
        Source::GitHub
            .allows(&Url::parse("https://release-assets.githubusercontent.com/f").unwrap())
    );
    for url in [
        "http://github.com/file",
        "https://github.com:8443/file",
        "https://github.com.evil.example/file",
        "https://api.github.com@evil.example/file",
        "https://token@github.com/file",
    ] {
        assert!(!Source::GitHub.allows(&Url::parse(url).unwrap()));
    }

    let source = Source::local("http://127.0.0.1:4444").unwrap();
    assert!(source.allows(&Url::parse("http://127.0.0.1:4444/file").unwrap()));
    assert!(!source.allows(&Url::parse("http://127.0.0.1:4445/file").unwrap()));
    assert!(Source::local("http://[::1]:4444").is_ok());
    for url in [
        "http://localhost:4444",
        "http://127.0.0.1",
        "http://127.0.0.1:4444/file",
        "https://127.0.0.1:4444",
        "http://127.0.0.1:4444?key=value",
        "http://0.0.0.0:4444",
    ] {
        assert!(Source::local(url).is_err());
    }
}

#[test]
fn verifies_download_and_rechecks_the_staged_file() {
    let (source, server) = serve_package(INSTALLER, INSTALLER.len());
    let directory = tempfile::tempdir().unwrap();
    let update = available(source);
    let download = download(&update, directory.path()).unwrap();
    server.join().unwrap();

    assert_eq!(download.path().file_name().unwrap(), INSTALLER_NAME);
    assert_eq!(fs::read(download.path()).unwrap(), INSTALLER);
    download.verify(download.path()).unwrap();

    fs::write(download.path(), b"modified package").unwrap();
    assert!(download.verify(download.path()).is_err());
    download.remove().unwrap();
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn rejects_corrupted_or_interrupted_downloads_and_removes_partial_files() {
    let directory = tempfile::tempdir().unwrap();
    for bytes in [b"modified package".as_slice(), b"partial".as_slice()] {
        let (source, server) = serve_package(bytes, INSTALLER.len());
        assert!(download(&available(source), directory.path()).is_err());
        server.join().unwrap();
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}

#[test]
fn reports_an_offline_feed_as_an_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = Source::local(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    drop(listener);

    assert!(check_from([2, 0, 0], Target::LinuxAppImage, "x86_64", source).is_err());
}

#[test]
fn checksum_requires_the_exact_filename_and_a_single_sha256() {
    let digest = hex::encode(Sha256::digest(INSTALLER));
    for invalid in [
        format!("{digest}  wrong.AppImage\n"),
        format!("{digest}  {INSTALLER_NAME}\n{digest}  other.AppImage\n"),
        format!("xx  {INSTALLER_NAME}\n"),
        format!("{digest}  /tmp/{INSTALLER_NAME}\n"),
    ] {
        assert!(parse_checksum(invalid.as_bytes(), INSTALLER_NAME).is_err());
    }
    assert!(
        parse_checksum(
            format!("{digest} *{INSTALLER_NAME}\r\n").as_bytes(),
            INSTALLER_NAME
        )
        .is_ok()
    );
}

fn available(source: Source) -> Update {
    check_from([2, 0, 0], Target::LinuxAppImage, "x86_64", source)
        .unwrap()
        .unwrap()
}

fn serve_package(bytes: &'static [u8], content_length: usize) -> (Source, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = Source::local(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let thread = thread::spawn(move || {
        let digest = hex::encode(Sha256::digest(INSTALLER));
        let checksum = format!("{digest}  {INSTALLER_NAME}\n");
        let checksum_name = format!("{INSTALLER_NAME}.sha256");
        let release_list = serde_json::to_vec(&vec![release(
            "v2.0.1",
            false,
            false,
            vec![
                asset(INSTALLER_NAME, INSTALLER.len()),
                asset(&checksum_name, checksum.len()),
            ],
        )])
        .unwrap();
        let expected_responses = [
            (
                format!("/{RELEASE_PATH}?per_page=100&page=1"),
                release_list.as_slice(),
                release_list.len(),
            ),
            (
                format!("/{DOWNLOAD_PATH}/v2.0.1/{checksum_name}"),
                checksum.as_bytes(),
                checksum.len(),
            ),
            (
                format!("/{DOWNLOAD_PATH}/v2.0.1/{INSTALLER_NAME}"),
                bytes,
                content_length,
            ),
        ];
        for (path, body, size) in expected_responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut request = BufReader::new(&stream);
            let mut line = String::new();
            request.read_line(&mut line).unwrap();
            assert_eq!(line.split_ascii_whitespace().nth(1).unwrap(), path);
            loop {
                line.clear();
                let read = request.read_line(&mut line).unwrap();
                if line == "\r\n" || read == 0 {
                    break;
                }
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            stream.write_all(body).unwrap();
        }
    });
    (source, thread)
}
