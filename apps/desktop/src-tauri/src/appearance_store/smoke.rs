//! Isolated real-package fixture; no user database or IPC entry point.
use super::AppearanceStoreState;
use crate::plugin_native::ModuleLaunch;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub(crate) struct Package {
    app: AppHandle,
    source: PathBuf,
    id: String,
}

impl Package {
    pub(crate) fn import(source: &Path, directory: &Path, app: &AppHandle) -> Result<Self, String> {
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(source.join("manifest.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let id = manifest["id"]
            .as_str()
            .ok_or("fixture plugin id missing")?
            .to_owned();
        let state = AppearanceStoreState::for_test(directory.join("settings.redb"));
        state.with_store(|store| store.import_plugin_directory_path(source))?;
        if !app.manage(state) {
            return Err("smoke store already registered".into());
        }
        Ok(Self {
            app: app.clone(),
            source: source.into(),
            id,
        })
    }

    pub(crate) fn launch(&self) -> Result<ModuleLaunch, String> {
        self.app
            .state::<AppearanceStoreState>()
            .native_launch(&self.id, "enhance")
    }

    pub(crate) fn action(&self, action: &str) -> Result<(), String> {
        self.app
            .state::<AppearanceStoreState>()
            .with_store(|store| match action {
                "disable" => store.set_plugin_enabled(&self.id, false).map(|_| ()),
                "upgrade" => store.import_plugin_directory_path(&self.source).map(|_| ()),
                "uninstall" => store.uninstall_plugin(&self.id).map(|_| ()),
                _ => Err("unknown package smoke action".into()),
            })
    }
}
