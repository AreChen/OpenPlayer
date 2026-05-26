use std::{fs, path::Path};

use redb::ReadableTable;

use crate::appearance_store::{
    PLUGIN_ENABLEMENT, PLUGIN_INSTALLS, PLUGIN_MANIFEST_FILE, PLUGIN_MANIFESTS,
    PLUGIN_PACKAGE_EXTENSION, THEME_MANIFESTS,
    manifest::parse_theme_plugin_manifest_json,
    package::{
        copy_directory_contents, extract_plugin_package, read_manifest_from_plugin_package,
        replace_directory_with_writer,
    },
    records::{current_time_ms, theme_manifests_for_plugin},
    store::AppearanceStore,
    types::{AppearanceState, PluginManifest, StoredPluginInstall, StoredThemeManifest},
};

impl AppearanceStore {
    #[cfg(test)]
    pub(in crate::appearance_store) fn import_theme_plugin_json(
        &mut self,
        json: &str,
    ) -> Result<AppearanceState, String> {
        let manifest = parse_theme_plugin_manifest_json(json)?;
        self.store_plugin_manifest(manifest, None)
    }

    pub(in crate::appearance_store) fn import_plugin_manifest_path(
        &mut self,
        path: &Path,
    ) -> Result<AppearanceState, String> {
        let json = fs::read_to_string(path)
            .map_err(|error| format!("failed to read plugin manifest: {error}"))?;
        let manifest = parse_theme_plugin_manifest_json(&json)?;
        let install_directory = self.plugin_install_directory(&manifest.id);
        let staging_directory = self.plugin_staging_directory(&manifest.id);
        replace_directory_with_writer(&install_directory, &staging_directory, |directory| {
            fs::write(directory.join(PLUGIN_MANIFEST_FILE), json.as_bytes())
                .map_err(|error| format!("failed to install plugin manifest: {error}"))
        })?;
        let record = StoredPluginInstall {
            package_kind: "manifestFile".to_string(),
            install_path: install_directory.to_string_lossy().to_string(),
            installed_at_ms: current_time_ms(),
        };
        self.store_plugin_manifest(manifest, Some(record))
    }

    pub(in crate::appearance_store) fn import_plugin_directory_path(
        &mut self,
        path: &Path,
    ) -> Result<AppearanceState, String> {
        if !path.is_dir() {
            return Err("plugin directory path must point to a directory".to_string());
        }
        let manifest_path = path.join(PLUGIN_MANIFEST_FILE);
        let json = fs::read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read plugin directory manifest: {error}"))?;
        let manifest = parse_theme_plugin_manifest_json(&json)?;
        let install_directory = self.plugin_install_directory(&manifest.id);
        let staging_directory = self.plugin_staging_directory(&manifest.id);
        replace_directory_with_writer(&install_directory, &staging_directory, |directory| {
            copy_directory_contents(path, directory)
        })?;
        let record = StoredPluginInstall {
            package_kind: "directory".to_string(),
            install_path: install_directory.to_string_lossy().to_string(),
            installed_at_ms: current_time_ms(),
        };
        self.store_plugin_manifest(manifest, Some(record))
    }

    pub(in crate::appearance_store) fn import_plugin_package_path(
        &mut self,
        path: &Path,
    ) -> Result<AppearanceState, String> {
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case(PLUGIN_PACKAGE_EXTENSION))
            != Some(true)
        {
            return Err("plugin package must use the .opplugin extension".to_string());
        }
        let json = read_manifest_from_plugin_package(path)?;
        let manifest = parse_theme_plugin_manifest_json(&json)?;
        let install_directory = self.plugin_install_directory(&manifest.id);
        let staging_directory = self.plugin_staging_directory(&manifest.id);
        replace_directory_with_writer(&install_directory, &staging_directory, |directory| {
            extract_plugin_package(path, directory)
        })?;
        let record = StoredPluginInstall {
            package_kind: "opplugin".to_string(),
            install_path: install_directory.to_string_lossy().to_string(),
            installed_at_ms: current_time_ms(),
        };
        self.store_plugin_manifest(manifest, Some(record))
    }

    pub(in crate::appearance_store) fn store_plugin_manifest(
        &mut self,
        manifest: PluginManifest,
        install: Option<StoredPluginInstall>,
    ) -> Result<AppearanceState, String> {
        let encoded_plugin = serde_json::to_string(&manifest)
            .map_err(|error| format!("failed to encode plugin manifest: {error}"))?;
        let encoded_install = install
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| format!("failed to encode plugin install record: {error}"))?;
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to import plugin: {error}"))?;
        {
            let mut plugin_manifests = transaction
                .open_table(PLUGIN_MANIFESTS)
                .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
            let mut theme_manifests = transaction
                .open_table(THEME_MANIFESTS)
                .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
            let mut plugin_enablement = transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
            let mut plugin_installs = transaction
                .open_table(PLUGIN_INSTALLS)
                .map_err(|error| format!("failed to open plugin installs table: {error}"))?;

            let stale_theme_ids = theme_manifests_for_plugin(&theme_manifests, &manifest.id)?;
            for theme_id in stale_theme_ids {
                theme_manifests
                    .remove(theme_id.as_str())
                    .map_err(|error| format!("failed to replace theme manifest: {error}"))?;
            }

            plugin_manifests
                .insert(manifest.id.as_str(), encoded_plugin.as_str())
                .map_err(|error| format!("failed to store plugin manifest: {error}"))?;
            if plugin_enablement
                .get(manifest.id.as_str())
                .map_err(|error| format!("failed to read plugin enablement: {error}"))?
                .is_none()
            {
                plugin_enablement
                    .insert(manifest.id.as_str(), "true")
                    .map_err(|error| format!("failed to store plugin enablement: {error}"))?;
            }
            if let Some(encoded_install) = encoded_install.as_deref() {
                plugin_installs
                    .insert(manifest.id.as_str(), encoded_install)
                    .map_err(|error| format!("failed to store plugin install record: {error}"))?;
            }

            for theme in &manifest.contributes.themes {
                let stored = StoredThemeManifest {
                    plugin_id: manifest.id.clone(),
                    theme: theme.clone(),
                };
                let encoded_theme = serde_json::to_string(&stored)
                    .map_err(|error| format!("failed to encode theme manifest: {error}"))?;
                theme_manifests
                    .insert(theme.id.as_str(), encoded_theme.as_str())
                    .map_err(|error| format!("failed to store theme manifest: {error}"))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin import: {error}"))?;
        self.state()
    }
}
