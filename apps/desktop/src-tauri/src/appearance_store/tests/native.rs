use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};

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
