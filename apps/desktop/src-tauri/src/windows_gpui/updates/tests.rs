use super::*;

fn release(tag_name: &str) -> Release {
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    Release {
        tag_name: tag_name.to_owned(),
        draft: false,
        prerelease: false,
        assets: vec![Asset {
            name: format!(
                "OpenPlayer_{}_{arch}-setup.exe",
                tag_name.trim_start_matches('v')
            ),
            state: "uploaded".into(),
        }],
    }
}

#[test]
fn only_offers_a_newer_stable_installer_in_the_installed_major() {
    let mut draft = release("v2.50.0");
    draft.draft = true;
    let mut prerelease = release("v2.40.0");
    prerelease.prerelease = true;
    let mut uploading = release("v2.30.0");
    uploading.assets[0].state = "new".into();
    let mut other_arch = release("v2.20.0");
    other_arch.assets[0].name = "OpenPlayer_2.20.0_amd64.deb".into();

    let (_, update) = latest_update(
        [
            release("v3.0.0"),
            release("v1.99.99"),
            draft,
            prerelease,
            uploading,
            other_arch,
            release("v2.100.0-rc.1"),
            release("v2.10.0"),
            release("v2.9.0"),
        ],
        [2, 0, 0],
    )
    .expect("published 2.x installer");

    assert_eq!(update.version, "2.10.0");
    assert!(
        update
            .url
            .starts_with(&format!("{RELEASE_DOWNLOADS}/v2.10.0/OpenPlayer_2.10.0_"))
    );
    assert!(update.url.ends_with("-setup.exe"));
    assert!(latest_update([release("v2.0.0"), release("v3.0.0")], [2, 0, 0]).is_none());
    assert!(latest_update([release("v2.0.0"), release("v1.6.9")], [1, 6, 10]).is_none());
}

#[test]
fn only_accepts_three_numeric_tag_components() {
    assert_eq!(stable_version("v2.5.10"), Some([2, 5, 10]));
    for invalid in [
        "v2.5",
        "v2.5.10.1",
        "v2.+5.10",
        "v2.05.10",
        "v2.5.10+dev",
        "v2.5.10/asset",
    ] {
        assert_eq!(stable_version(invalid), None, "{invalid}");
    }
}
