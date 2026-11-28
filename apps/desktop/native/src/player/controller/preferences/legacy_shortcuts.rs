use std::{
    collections::BTreeMap,
    fs::{self, File},
    io,
    path::Path,
};

use anyhow::Result;
use rusty_leveldb::{DB, Options, in_memory};
use serde_json::{Map, Value};

use super::{DEFAULT_CHORDS, default_chords};

const SHORTCUT_KEY: &[u8] = b"\x01openplayer.shortcuts.v3";
const ORIGINS: [&[u8]; 2] = [b"_http://tauri.localhost\0", b"_https://tauri.localhost\0"];

pub(super) fn read(source: &Path) -> Result<Option<BTreeMap<String, String>>> {
    let current = source.join("CURRENT");
    if !current.is_file() {
        return Ok(None);
    }
    let manifest = fs::read(&current)?;
    let options = Options {
        create_if_missing: false,
        ..in_memory()
    };
    let snapshot = Path::new("openplayer-legacy-shortcuts");

    // LevelDB recovery can rewrite logs. Run it against an in-memory copy of the old store.
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(text) = name.to_str() else { continue };
        if entry.file_type()?.is_file() && database_file(text) {
            let mut original = File::open(entry.path())?;
            let mut copy = options.env.open_writable_file(&snapshot.join(name))?;
            io::copy(&mut original, &mut copy)?;
        }
    }
    anyhow::ensure!(
        fs::read(&current)? == manifest,
        "the legacy shortcut database changed while opening it"
    );
    read_record(&mut DB::open(snapshot, options)?)
}

fn database_file(name: &str) -> bool {
    fn file_number(value: &str) -> bool {
        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
    }

    name == "CURRENT"
        || name.strip_prefix("MANIFEST-").is_some_and(file_number)
        || name.rsplit_once('.').is_some_and(|(number, extension)| {
            file_number(number) && matches!(extension, "log" | "ldb" | "sst")
        })
}

fn read_record(database: &mut DB) -> Result<Option<BTreeMap<String, String>>> {
    let snapshot = database.get_snapshot();
    for origin in ORIGINS {
        let key = [origin, SHORTCUT_KEY].concat();
        if let Some(record) = database.get_at(&snapshot, &key)? {
            return decode_bindings(&record).map(Some);
        }
    }
    Ok(None)
}

fn decode_bindings(record: &[u8]) -> Result<BTreeMap<String, String>> {
    let json = match record.split_first() {
        Some((1, latin1)) => latin1.iter().copied().map(char::from).collect::<String>(),
        Some((0, utf16)) if utf16.len().is_multiple_of(2) => String::from_utf16(
            &utf16
                .as_chunks::<2>()
                .0
                .iter()
                .copied()
                .map(u16::from_le_bytes)
                .collect::<Vec<_>>(),
        )?,
        _ => anyhow::bail!("unrecognized legacy shortcut encoding"),
    };
    let stored: Map<String, Value> = serde_json::from_str(&json)?;
    let mut bindings = default_chords();
    for (action, _) in DEFAULT_CHORDS {
        match stored.get(action) {
            Some(Value::Null) => {
                bindings.remove(action);
            }
            Some(Value::String(chord))
                if !chord.is_empty() && !chord.chars().any(char::is_control) =>
            {
                bindings.retain(|_, current| current != chord);
                bindings.insert(action.to_owned(), chord.clone());
            }
            _ => {}
        }
    }
    Ok(bindings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_only_player_bindings_and_keeps_explicit_unsets() {
        let options = in_memory();
        let fixture = Path::new("openplayer-test");
        let mut database = DB::open(fixture, options.clone()).unwrap();
        let key = [ORIGINS[0], SHORTCUT_KEY].concat();
        database
            .put(
                &key,
                b"\x01{\"openMedia\":\"Ctrl+R\",\"restart\":null,\"volumeUp\":\"ArrowDown\",\"plugin:example:action\":\"K\"}",
            )
            .unwrap();
        database.close().unwrap();

        let directory = std::env::temp_dir().join(format!(
            "openplayer-legacy-shortcuts-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&directory).unwrap();
        let mut originals = BTreeMap::new();
        for file in options.env.children(fixture).unwrap() {
            let name = file.file_name().unwrap();
            if database_file(name.to_str().unwrap()) {
                let mut bytes = Vec::new();
                io::copy(
                    &mut options
                        .env
                        .open_sequential_file(&fixture.join(name))
                        .unwrap(),
                    &mut bytes,
                )
                .unwrap();
                let destination = directory.join(name);
                fs::write(&destination, &bytes).unwrap();
                originals.insert(destination, bytes);
            }
        }
        let result = read(&directory);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), originals.len());
        for (file, expected) in originals {
            assert_eq!(fs::read(&file).unwrap(), expected);
            fs::remove_file(file).unwrap();
        }
        fs::remove_dir(directory).unwrap();

        let bindings = result.unwrap().unwrap();
        assert_eq!(bindings["openMedia"], "Ctrl+R");
        assert_eq!(bindings["volumeUp"], "ArrowDown");
        assert_eq!(bindings["seekForward"], "ArrowRight");
        assert!(!bindings.contains_key("restart"));
        assert!(!bindings.contains_key("volumeDown"));
        assert!(!bindings.contains_key("plugin:example:action"));
    }

    #[test]
    fn reads_utf16_bindings_and_accepts_only_leveldb_files() {
        let json = r#"{"openMedia":"Ctrl+中"}"#;
        let encoded: Vec<_> = std::iter::once(0)
            .chain(json.encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode_bindings(&encoded).unwrap()["openMedia"], "Ctrl+中");
        for name in [
            "CURRENT",
            "MANIFEST-000001",
            "000003.log",
            "000004.ldb",
            "000005.sst",
        ] {
            assert!(database_file(name));
        }
        for name in ["LOCK", "LOG", "LOG.old", "profile.log", "MANIFEST-sample"] {
            assert!(!database_file(name));
        }
    }
}
