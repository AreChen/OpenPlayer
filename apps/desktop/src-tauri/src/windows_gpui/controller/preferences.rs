mod legacy_shortcuts;

use std::{collections::BTreeMap, fs, path::PathBuf};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

const SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("settings_kv");

pub(super) const ACCENT: &str = "accentOverride";
pub(super) const LANGUAGE: &str = "languageMode";
pub(super) const INCOGNITO: &str = "incognitoMode";
pub(super) const QUIET_KEYBOARD: &str = "quietKeyboardControls";
pub(super) const TITLEBAR_OPACITY: &str = "titlebarOpacity";
pub(super) const TRANSPORT_OPACITY: &str = "transportOpacity";
pub(super) const SHORTCUTS: &str = "nativeShortcutBindings";
pub(super) const FAVORITE_DIRECTORIES: &str = "mediaFavoriteDirectories";
pub(super) const FAVORITE_MEDIA: &str = "mediaFavoriteFiles";
pub(super) const LAST_MEDIA_DIRECTORY: &str = "lastMediaBrowserDirectory";
pub(super) const DEFAULT_TITLEBAR_OPACITY: f64 = 0.42;
pub(super) const DEFAULT_TRANSPORT_OPACITY: f64 = 0.54;

const FIT_WINDOW: (&str, &str) = ("fitWindowToVideo", "Ctrl+H");

pub(super) const DEFAULT_CHORDS: [(&str, &str); 14] = [
    ("openMedia", "Ctrl+O"),
    ("togglePlayback", "Space"),
    ("restart", "R"),
    ("togglePlaylist", "P"),
    ("seekBackward", "ArrowLeft"),
    ("seekForward", "ArrowRight"),
    ("frameBackward", "D"),
    ("frameForward", "F"),
    ("volumeDown", "ArrowDown"),
    ("volumeUp", "ArrowUp"),
    ("toggleFullscreen", "Enter"),
    FIT_WINDOW,
    ("toggleAlwaysOnTop", "\\"),
    ("openSettings", "Ctrl+,"),
];

pub(super) fn default_chords() -> BTreeMap<String, String> {
    DEFAULT_CHORDS
        .into_iter()
        .map(|(action, chord)| (action.to_owned(), chord.to_owned()))
        .collect()
}

fn decode_chords(value: &str) -> BTreeMap<String, String> {
    let Ok(mut stored) = serde_json::from_str::<BTreeMap<String, Option<String>>>(value) else {
        return default_chords();
    };
    let (action, chord) = FIT_WINDOW;
    // Older native profiles have no entry for the new action. A saved null is an explicit unbind.
    if !stored.contains_key(action) && !stored.values().any(|value| value.as_deref() == Some(chord))
    {
        stored.insert(action.to_owned(), Some(chord.to_owned()));
    }
    stored
        .into_iter()
        .filter_map(|(action, chord)| chord.map(|chord| (action, chord)))
        .collect()
}

fn encode_chords(bindings: &BTreeMap<String, String>) -> serde_json::Result<String> {
    let mut stored: BTreeMap<_, _> = bindings
        .iter()
        .map(|(action, chord)| (action.as_str(), Some(chord.as_str())))
        .collect();
    stored.entry(FIT_WINDOW.0).or_insert(None);
    serde_json::to_string(&stored)
}

pub(super) fn storage_directory() -> PathBuf {
    let profile = match std::env::var_os("OPENPLAYER_WINDOWS_SMOKE_PROFILE") {
        Some(value) => match value.to_str() {
            Some(name)
                if !name.is_empty()
                    && name != "1"
                    && name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
                    }) =>
            {
                format!("dev.openplayer.gpui-smoke-{name}")
            }
            _ => "dev.openplayer.gpui-smoke".to_owned(),
        },
        None => "dev.openplayer.app".to_owned(),
    };
    std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(profile)
        .join("storage")
}

pub(super) struct Preferences {
    path: PathBuf,
    pub(super) pending: BTreeMap<&'static str, Option<String>>,
}

impl Preferences {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            pending: BTreeMap::new(),
        }
    }

    pub(super) fn read(&self) -> anyhow::Result<BTreeMap<&'static str, String>> {
        if !self.path.is_file() {
            return Ok(BTreeMap::new());
        }
        let database = Database::open(&self.path)?;
        let transaction = database.begin_read()?;
        let table = match transaction.open_table(SETTINGS) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(BTreeMap::new()),
            Err(error) => return Err(error.into()),
        };
        let mut values = BTreeMap::new();
        for key in [
            ACCENT,
            LANGUAGE,
            INCOGNITO,
            QUIET_KEYBOARD,
            TITLEBAR_OPACITY,
            TRANSPORT_OPACITY,
            SHORTCUTS,
            FAVORITE_DIRECTORIES,
            FAVORITE_MEDIA,
            LAST_MEDIA_DIRECTORY,
        ] {
            if let Some(value) = table.get(key)? {
                values.insert(key, value.value().to_owned());
            }
        }
        Ok(values)
    }

    pub(super) fn set(&mut self, key: &'static str, value: Option<String>) {
        self.pending.insert(key, value);
    }

    pub(super) fn load_shortcuts(&self, stored: Option<&str>) -> BTreeMap<String, String> {
        if let Some(value) = stored {
            return decode_chords(value);
        }
        if std::env::var_os("OPENPLAYER_WINDOWS_SMOKE_PROFILE").is_none()
            && let Some(path) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        {
            let source = path.join(r"dev.openplayer.app\EBWebView\Default\Local Storage\leveldb");
            match legacy_shortcuts::read(&source) {
                Ok(Some(bindings)) => match self.store_imported_shortcuts(&bindings) {
                    Ok(effective) => return effective,
                    Err(error) => {
                        eprintln!("OpenPlayer shortcut migration: {error:#}");
                        return bindings;
                    }
                },
                Ok(None) => {}
                Err(error) => eprintln!("OpenPlayer shortcut migration: {error:#}"),
            }
        }
        default_chords()
    }

    pub(super) fn save_shortcuts(
        &mut self,
        bindings: &BTreeMap<String, String>,
    ) -> serde_json::Result<()> {
        self.set(SHORTCUTS, Some(encode_chords(bindings)?));
        Ok(())
    }

    fn store_imported_shortcuts(
        &self,
        bindings: &BTreeMap<String, String>,
    ) -> anyhow::Result<BTreeMap<String, String>> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let database = Database::create(&self.path)?;
        let transaction = database.begin_write()?;
        {
            let mut table = transaction.open_table(SETTINGS)?;
            let existing = table.get(SHORTCUTS)?.map(|value| value.value().to_owned());
            if let Some(value) = existing {
                return Ok(decode_chords(&value));
            }
            let serialized = encode_chords(bindings)?;
            table.insert(SHORTCUTS, serialized.as_str())?;
        }
        transaction.commit()?;
        Ok(bindings.clone())
    }

    pub(super) fn flush(&mut self) -> anyhow::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let database = Database::create(&self.path)?;
        let transaction = database.begin_write()?;
        {
            let mut table = transaction.open_table(SETTINGS)?;
            for (&key, value) in &self.pending {
                if let Some(value) = value {
                    table.insert(key, value.as_str())?;
                } else {
                    table.remove(key)?;
                }
            }
        }
        transaction.commit()?;
        self.pending.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplies_the_new_fit_shortcut_to_existing_profiles() {
        let chords = decode_chords(r#"{"openMedia":"Ctrl+M"}"#);
        assert_eq!(chords["openMedia"], "Ctrl+M");
        assert_eq!(chords[FIT_WINDOW.0], FIT_WINDOW.1);
        assert_eq!(decode_chords("{}")[FIT_WINDOW.0], FIT_WINDOW.1);
    }

    #[test]
    fn keeps_conflicting_and_explicitly_unbound_fit_shortcuts() {
        let chords = decode_chords(r#"{"openMedia":"Ctrl+H"}"#);
        assert!(!chords.contains_key(FIT_WINDOW.0));
        assert_eq!(chords["openMedia"], FIT_WINDOW.1);

        let mut chords = default_chords();
        chords.remove(FIT_WINDOW.0);
        let saved = encode_chords(&chords).unwrap();
        assert_eq!(decode_chords(&saved), chords);
    }
}
