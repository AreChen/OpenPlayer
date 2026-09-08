use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};

#[test]
#[cfg(windows)]
fn plugin_directory_removal_waits_for_transient_windows_file_lock() {
    use std::os::windows::fs::OpenOptionsExt;
    let (store, directory) = temp_store();
    let plugin = store.plugin_root.join("lock-fixture");
    std::fs::create_dir(&plugin).unwrap();
    let locked = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(1 | 2)
        .open(plugin.join("worker.dll"))
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        drop(locked);
    });
    super::super::package::remove_installed_plugin_directory(&store.plugin_root, &plugin).unwrap();
    release.join().unwrap();
    assert!(!plugin.exists());
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(windows)]
fn plugin_directory_removal_does_not_retry_a_file_lock_forever() {
    use std::os::windows::fs::OpenOptionsExt;
    let (store, directory) = temp_store();
    let plugin = store.plugin_root.join("persistent-lock-fixture");
    std::fs::create_dir(&plugin).unwrap();
    let locked = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(1 | 2)
        .open(plugin.join("worker.dll"))
        .unwrap();
    assert!(
        super::super::package::remove_installed_plugin_directory(&store.plugin_root, &plugin)
            .is_err()
    );
    assert!(plugin.exists());
    drop(locked);
    super::super::package::remove_installed_plugin_directory(&store.plugin_root, &plugin).unwrap();
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(windows)]
#[ignore = "requires an explicitly staged portable module and authorized local NVIDIA runtime"]
fn portable_frame_module_runs_after_real_package_import() {
    let source = PathBuf::from(
        std::env::var_os("OPENPLAYER_NATIVE_FRAME_PACKAGE").expect("provide portable package"),
    );
    assert!(source.is_absolute());
    let (mut store, directory) = temp_store();
    store.import_plugin_directory_path(&source).unwrap();
    drop(store);
    let state = super::super::AppearanceStoreState::for_test(directory.join("settings.redb"));
    let plugin_id = "dev.openplayer.dlssnr.portable";
    let launch = state.native_launch(plugin_id, "enhance").unwrap();
    assert!(
        launch
            .executable
            .starts_with(directory.canonicalize().unwrap())
    );
    assert!(
        !launch
            .executable
            .starts_with(source.canonicalize().unwrap())
    );
    let installed_root = launch
        .executable
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    crate::plugin_native::tests::exercise_portable_frame_owner(launch, |action| {
        state
            .with_store(|store| match action {
                "disable" => {
                    store.set_plugin_enabled(plugin_id, false)?;
                    store.set_plugin_enabled(plugin_id, true)?;
                    Ok(())
                }
                "upgrade" => store.import_plugin_directory_path(&source).map(|_| ()),
                "uninstall" => store.uninstall_plugin(plugin_id).map(|_| ()),
                _ => unreachable!(),
            })
            .unwrap();
    });
    assert!(state.native_launch(plugin_id, "enhance").is_err());
    assert!(
        !installed_root.exists(),
        "installed runtime files must not remain locked"
    );
    drop(state);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(windows)]
#[ignore = "requires explicit Python, prototype checkout and authorized local NVIDIA runtime"]
fn native_frame_owner_follows_real_plugin_lifecycle() {
    let (mut store, directory) = temp_store();
    let source = directory.join("source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("worker.exe"),
        b"native lifecycle package fixture",
    )
    .unwrap();
    let mut manifest = json!({
        "id":"test.native.frame.owner", "name":"Frame owner fixture", "version":"1.0.0", "entry":"manifest",
        "contributes": {
            "capabilities":[{"id":"native", "name":"Native", "kind":"nativeTool", "permissions":["native.process"]}],
            "nativeModules":[{"id":"enhance", "protocol":"openplayer-native-v1", "methods":["frames.open","frames.status","frames.close"],
                "targets":{"windows-x86_64":{"entry":"worker.exe","sha256":format!("{:x}",Sha256::digest(b"native lifecycle package fixture"))}}}]
        }
    });
    std::fs::write(source.join("manifest.json"), manifest.to_string()).unwrap();
    store.import_plugin_directory_path(&source).unwrap();
    crate::plugin_native::tests::exercise_frame_owner_lifecycle(|action| match action {
        "disable" => {
            store
                .set_plugin_enabled("test.native.frame.owner", false)
                .unwrap();
            assert!(
                !store
                    .state()
                    .unwrap()
                    .plugins
                    .iter()
                    .find(|plugin| plugin.id == "test.native.frame.owner")
                    .unwrap()
                    .enabled
            );
            store
                .set_plugin_enabled("test.native.frame.owner", true)
                .unwrap();
        }
        "upgrade" => {
            manifest["version"] = json!("2.0.0");
            std::fs::write(source.join("manifest.json"), manifest.to_string()).unwrap();
            store.import_plugin_directory_path(&source).unwrap();
            assert_eq!(
                store
                    .plugin_manifest("test.native.frame.owner")
                    .unwrap()
                    .version,
                "2.0.0"
            );
        }
        "uninstall" => {
            store.uninstall_plugin("test.native.frame.owner").unwrap();
        }
        _ => unreachable!(),
    });
    assert!(
        store
            .plugin_install_record("test.native.frame.owner")
            .unwrap()
            .is_none()
    );
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_packages_validate_before_replacing_existing_install() {
    let (mut store, directory) = temp_store();
    let source = directory.join("source");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("worker.exe"), b"native fixture").unwrap();
    let mut manifest = json!({
        "id":"test.native.package", "name":"Native fixture", "version":"1.0.0", "entry":"manifest",
        "contributes": {
            "capabilities":[{"id":"native", "name":"Native", "kind":"nativeTool", "permissions":["native.process"]}],
            "nativeModules":[{"id":"enhance", "protocol":"openplayer-native-v1", "methods":["echo"],
                "targets":{"windows-x86_64":{"entry":"worker.exe","sha256":format!("{:x}",Sha256::digest(b"native fixture"))}}}]
        }
    });
    std::fs::write(source.join("manifest.json"), manifest.to_string()).unwrap();
    assert!(
        store
            .import_plugin_manifest_path(&source.join("manifest.json"))
            .is_err()
    );
    store.import_plugin_directory_path(&source).unwrap();
    let installed = store
        .plugin_install_record("test.native.package")
        .unwrap()
        .unwrap();
    let installed_file = PathBuf::from(installed.install_path).join("worker.exe");
    assert_eq!(std::fs::read(&installed_file).unwrap(), b"native fixture");
    manifest["version"] = json!("2.0.0");
    std::fs::write(source.join("manifest.json"), manifest.to_string()).unwrap();
    std::fs::write(source.join("worker.exe"), b"tampered fixture").unwrap();
    assert!(
        store
            .import_plugin_directory_path(&source)
            .unwrap_err()
            .contains("integrity")
    );
    assert_eq!(
        store
            .plugin_manifest("test.native.package")
            .unwrap()
            .version,
        "1.0.0"
    );
    assert_eq!(std::fs::read(&installed_file).unwrap(), b"native fixture");
    store.uninstall_plugin("test.native.package").unwrap();
    assert!(!installed_file.exists());
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}
