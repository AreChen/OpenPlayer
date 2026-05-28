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
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to initialize appearance settings: {error}"))?;
        {
            transaction
                .open_table(SETTINGS_KV)
                .map_err(|error| format!("failed to open appearance settings table: {error}"))?;
            transaction
                .open_table(THEME_MANIFESTS)
                .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
            transaction
                .open_table(PLUGIN_MANIFESTS)
                .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
            transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
            transaction
                .open_table(PLUGIN_SETTINGS)
                .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
            transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            transaction
                .open_table(PLUGIN_RUNTIME_STORAGE_META)
                .map_err(|error| {
                    format!("failed to open plugin runtime storage metadata table: {error}")
                })?;
            transaction
                .open_table(PLUGIN_INSTALLS)
                .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
        }
        transaction.commit().map_err(|error| {
            format!("failed to commit appearance settings initialization: {error}")
        })
    }
}
