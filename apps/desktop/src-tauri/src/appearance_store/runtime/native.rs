use redb::ReadableDatabase;

use crate::{
    appearance_store::{
        AppearanceStoreState, PLUGIN_ENABLEMENT,
        package::resolve_plugin_runtime_script_path,
        records::{plugin_enabled_from_table, plugin_permissions},
    },
    plugin_native::{ModuleLaunch, NativeModule},
};

impl AppearanceStoreState {
    pub(crate) fn native_modules(&self, plugin_id: &str) -> Result<Vec<NativeModule>, String> {
        self.with_store(|store| {
            let manifest = store.plugin_manifest(plugin_id)?;
            if !plugin_permissions(&manifest)
                .iter()
                .any(|p| p == "native.process")
            {
                return Err("native modules require native.process".into());
            }
            let transaction = store.database.begin_read().map_err(|e| e.to_string())?;
            let table = transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|e| e.to_string())?;
            if !plugin_enabled_from_table(&table, plugin_id)? {
                return Err("native plugin is disabled".into());
            }
            Ok(manifest.contributes.native_modules)
        })
    }

    pub(crate) fn native_launch(
        &self,
        plugin_id: &str,
        module_id: &str,
    ) -> Result<ModuleLaunch, String> {
        // Commands sample the runtime generation before this snapshot and verify it
        // on registration, so disable/update cannot race a pending consent dialog.
        let modules = self.native_modules(plugin_id)?;
        let module = modules
            .into_iter()
            .find(|m| m.id == module_id)
            .ok_or("unknown native module")?;
        self.with_store(|store| {
            let manifest = store.plugin_manifest(plugin_id)?;
            let install = store
                .plugin_install_record(plugin_id)?
                .ok_or("plugin is not installed")?;
            let target = module
                .targets
                .get(&crate::plugin_native::current_target())
                .ok_or("native module does not support this platform")?;
            let executable =
                resolve_plugin_runtime_script_path(&install.install_path, &target.entry)?;
            crate::plugin_native::verify_executable(&executable, &target.sha256)?;
            Ok(ModuleLaunch {
                plugin_id: plugin_id.into(),
                plugin_name: manifest.name,
                plugin_version: manifest.version,
                executable,
                package_root: std::path::PathBuf::from(&install.install_path),
                language_mode: store.preferences()?.language_mode,
                args: target.args.clone(),
                module,
            })
        })
    }
}
