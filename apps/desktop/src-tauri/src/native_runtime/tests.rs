use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "openplayer-runtime-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn source(&self) -> PathBuf {
        let source = self.0.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("file.bin"), b"runtime fixture").unwrap();
        self.inventory(
            &source,
            "file.bin",
            15,
            &format!("{:x}", Sha256::digest(b"runtime fixture")),
        );
        source
    }
    fn inventory(&self, source: &Path, name: &str, size: u64, hash: &str) {
        fs::write(
            source.join("bundle-inventory.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema": 2, "files": {name: {"size": size, "sha256": hash}}
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn verified_copy_survives_source_removal_and_protects_live_lease() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let inventory = inventory::Inventory::read(&source).unwrap();
    let cache = fixture.0.join("cache");
    let mut copy = cache::RuntimeCopy::prepare(&source, &cache, &inventory).unwrap();
    fs::remove_dir_all(&source).unwrap();
    assert_eq!(
        fs::read(copy.path.join("file.bin")).unwrap(),
        b"runtime fixture"
    );
    copy.retain_until_exit();
    let content = copy.path.clone();
    cache::cleanup(&cache).unwrap();
    assert!(content.exists(), "live runtime lease must prevent deletion");
    drop(copy);
    cache::cleanup(&cache).unwrap();
    assert!(!content.exists(), "released runtime must be reclaimable");
}

#[test]
fn failed_copy_is_removed_and_corrupt_or_extra_files_are_rejected() {
    let fixture = Fixture::new();
    let source = fixture.source();
    let inventory = inventory::Inventory::read(&source).unwrap();
    let cache = fixture.0.join("cache");
    fs::write(source.join("file.bin"), b"changed fixture").unwrap();
    assert!(cache::RuntimeCopy::prepare(&source, &cache, &inventory).is_err());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
    fs::write(source.join("file.bin"), b"runtime fixture").unwrap();
    fs::write(source.join("extra.dll"), b"extra").unwrap();
    assert!(cache::RuntimeCopy::prepare(&source, &cache, &inventory).is_err());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);
}

#[test]
fn rejects_traversal_reserved_names_oversize_and_case_collisions() {
    let fixture = Fixture::new();
    let source = fixture.source();
    for name in [
        "../escape",
        "/escape",
        "C:/escape",
        "file:stream",
        "dir\\file",
        "NUL.dll",
        "dir./file",
        "dir//file",
    ] {
        fixture.inventory(&source, name, 1, &"0".repeat(64));
        assert!(inventory::Inventory::read(&source).is_err(), "{name}");
    }
    fixture.inventory(&source, "file", u64::MAX, &"0".repeat(64));
    assert!(inventory::Inventory::read(&source).is_err());
    fs::write(source.join("bundle-inventory.json"), format!(r#"{{"schema":2,"files":{{"A":{{"size":1,"sha256":"{0}"}},"a":{{"size":1,"sha256":"{0}"}}}}}}"#, "0".repeat(64))).unwrap();
    assert!(inventory::Inventory::read(&source).is_err());
}

#[test]
fn failed_load_does_not_pin_or_leave_copies() {
    let fixture = Fixture::new();
    let source = fixture.source();
    assert!(pin_vsscript(&source, &fixture.0.join("cache")).is_err());
    assert!(RESIDENT.lock().unwrap().is_none());
    let mut files = serde_json::Map::new();
    for name in [
        VSSCRIPT,
        "runtime/python.exe",
        "runtime/python3.dll",
        "runtime/python312.dll",
        "runtime/python312._pth",
        "file.bin",
    ] {
        let path = source.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"not a library").unwrap();
        files.insert(name.into(), serde_json::json!({"size": 13, "sha256": format!("{:x}", Sha256::digest(b"not a library"))}));
    }
    fs::write(
        source.join("bundle-inventory.json"),
        serde_json::to_vec(&serde_json::json!({"schema": 2, "files": files})).unwrap(),
    )
    .unwrap();
    let cache = fixture.0.join("cache");
    assert!(
        pin_vsscript(&source, &cache)
            .unwrap_err()
            .contains("runtime load failed")
    );
    assert!(RESIDENT.lock().unwrap().is_none());
    assert_eq!(fs::read_dir(cache).unwrap().count(), 0);
}

#[test]
fn stale_cleanup_preserves_unowned_directories() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.0.join("unrelated")).unwrap();
    fs::create_dir(fixture.0.join("runtime-invalid-name")).unwrap();
    cache::cleanup(&fixture.0).unwrap();
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 2);
}
