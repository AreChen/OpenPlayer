use std::{fs, path::PathBuf, sync::Mutex};

use redb::Database;
use tauri::{AppHandle, Manager};

use super::{
    PLUGIN_ENABLEMENT, PLUGIN_INSTALLS, PLUGIN_MANIFESTS, PLUGIN_RUNTIME_STORAGE,
    PLUGIN_RUNTIME_STORAGE_META, PLUGIN_SETTINGS, SETTINGS_KV, THEME_MANIFESTS,
    database::create_database_with_retry,
};
pub struct AppearanceStoreState {
    path: PathBuf,
    access: Mutex<()>,
}

pub(super) struct AppearanceStore {
    pub(super) database: Database,
    pub(super) plugin_root: PathBuf,
}

impl AppearanceStoreState {
    #[cfg(all(windows, any(test, feature = "window-smoke")))]
    pub(super) fn for_test(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    pub fn open(app: &AppHandle) -> Self {
        let path = match Self::store_path(app) {
            Ok(path) => path,
            Err(error) => {
                eprintln!("{error}");
                PathBuf::from("openplayer-settings.redb")
            }
        };

        Self {
            path,
            access: Mutex::new(()),
        }
    }

    pub(super) fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
        let mut directory = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
        directory.push("storage");
        Ok(directory.join("openplayer-settings.redb"))
    }

    pub(super) fn with_store<T>(
        &self,
        action: impl FnOnce(&mut AppearanceStore) -> Result<T, String>,
    ) -> Result<T, String> {
        let _guard = self
            .access
            .lock()
            .map_err(|_| "appearance store lock failed".to_string())?;
        let mut store = AppearanceStore::open(self.path.clone())?;
        action(&mut store)
    }
}

impl AppearanceStore {
    pub(super) fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("failed to create appearance settings directory: {error}")
            })?;
        }

        let plugin_root = path
            .parent()
            .map(|parent| parent.join("plugins"))
            .unwrap_or_else(|| PathBuf::from("plugins"));
        fs::create_dir_all(&plugin_root)
            .map_err(|error| format!("failed to create plugin directory: {error}"))?;

        let database = create_database_with_retry(&path, "appearance settings")?;
        let store = Self {
            database,
            plugin_root,
        };
        store.initialize()?;
        Ok(store)
    }

    pub(super) fn initialize(&self) -> Result<(), String> {
        crate::store_schema::ensure_string_tables(
            &self.database,
            &[
                SETTINGS_KV,
                THEME_MANIFESTS,
                PLUGIN_MANIFESTS,
                PLUGIN_ENABLEMENT,
                PLUGIN_SETTINGS,
                PLUGIN_RUNTIME_STORAGE,
                PLUGIN_RUNTIME_STORAGE_META,
                PLUGIN_INSTALLS,
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("failed to initialize store schema: {error}"))
    }
}
