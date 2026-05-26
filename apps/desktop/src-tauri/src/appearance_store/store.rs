use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use redb::{Database, ReadableDatabase, ReadableTable};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use super::{
    ACCENT_OVERRIDE_KEY, ACTIVE_THEME_KEY, DEFAULT_THEME_ID, INCOGNITO_MODE_KEY, LANGUAGE_MODE_KEY,
    MAX_PLUGIN_RUNTIME_SCRIPT_BYTES, MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES,
    MAX_PLUGIN_VIEW_HTML_BYTES, PLUGIN_ENABLEMENT, PLUGIN_INSTALLS, PLUGIN_MANIFEST_FILE,
    PLUGIN_MANIFESTS, PLUGIN_PACKAGE_EXTENSION, PLUGIN_RUNTIME_STORAGE, PLUGIN_SETTINGS,
    QUIET_KEYBOARD_CONTROLS_KEY, SETTINGS_KV, THEME_MANIFESTS,
    database::create_database_with_retry,
    manifest::{
        parse_theme_plugin_manifest_json, validate_color_token, validate_dotted_identifier,
        validate_plugin_setting_value,
    },
    package::{
        copy_directory_contents, extract_plugin_package, read_manifest_from_plugin_package,
        remove_installed_plugin_directory, replace_directory_with_writer,
        resolve_plugin_package_file_path, resolve_plugin_runtime_script_path,
    },
    records::{
        current_time_ms, decode_plugin_manifest, decode_plugin_runtime_storage_value,
        decode_stored_theme_manifest, plugin_action_summaries, plugin_capability_summaries,
        plugin_enabled_from_table, plugin_install_from_table, plugin_permissions,
        plugin_runtime_storage_key, plugin_runtime_storage_keys_for_plugin,
        plugin_runtime_storage_prefix, plugin_setting_key, plugin_setting_keys_for_plugin,
        plugin_setting_summaries, plugin_view_summaries, read_bool_setting,
        read_language_mode_setting, runtime_kind_label, theme_belongs_to_plugin,
        theme_manifests_for_plugin, validate_language_mode, validate_plugin_runtime_storage_key,
    },
    themes::built_in_theme_catalog,
    types::*,
};
pub struct AppearanceStoreState {
    path: PathBuf,
    access: Mutex<()>,
}

pub(super) struct AppearanceStore {
    database: Database,
    plugin_root: PathBuf,
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
                .open_table(PLUGIN_INSTALLS)
                .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
        }
        transaction.commit().map_err(|error| {
            format!("failed to commit appearance settings initialization: {error}")
        })
    }

    pub(super) fn state(&self) -> Result<AppearanceState, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read appearance settings: {error}"))?;
        let settings = transaction
            .open_table(SETTINGS_KV)
            .map_err(|error| format!("failed to open appearance settings table: {error}"))?;
        let plugin_manifests = transaction
            .open_table(PLUGIN_MANIFESTS)
            .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
        let theme_manifests = transaction
            .open_table(THEME_MANIFESTS)
            .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
        let plugin_enablement = transaction
            .open_table(PLUGIN_ENABLEMENT)
            .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
        let plugin_settings = transaction
            .open_table(PLUGIN_SETTINGS)
            .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
        let plugin_installs = transaction
            .open_table(PLUGIN_INSTALLS)
            .map_err(|error| format!("failed to open plugin installs table: {error}"))?;

        let mut plugins = Vec::new();
        for item in plugin_manifests
            .iter()
            .map_err(|error| format!("failed to scan plugin manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read plugin manifest: {error}"))?;
            let manifest = decode_plugin_manifest(value.value())?;
            let enabled = plugin_enabled_from_table(&plugin_enablement, &manifest.id)?;
            let settings = plugin_setting_summaries(&plugin_settings, &manifest)?;
            let setting_count = settings.len();
            let capabilities = plugin_capability_summaries(&manifest);
            let actions = plugin_action_summaries(&manifest);
            let views = plugin_view_summaries(&manifest);
            let permissions = plugin_permissions(&manifest);
            let install = plugin_install_from_table(&plugin_installs, &manifest.id)?;
            plugins.push(ThemePluginSummary {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
                api_version: manifest.api_version,
                min_host_version: manifest.min_host_version,
                author: manifest.author,
                update_url: manifest.update_url,
                description: manifest.description,
                enabled,
                package_kind: install
                    .as_ref()
                    .map(|install| install.package_kind.clone())
                    .unwrap_or_else(|| "legacyManifest".to_string()),
                install_path: install.as_ref().map(|install| install.install_path.clone()),
                installed_at_ms: install.as_ref().map(|install| install.installed_at_ms),
                theme_count: manifest.contributes.themes.len(),
                runtime: runtime_kind_label(&manifest.runtime.kind).to_string(),
                capability_count: manifest.contributes.capabilities.len(),
                setting_count,
                action_count: manifest.contributes.actions.len(),
                permissions,
                capabilities,
                settings,
                actions,
                views,
            });
        }
        plugins.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));

        let mut themes = built_in_theme_catalog();
        for item in theme_manifests
            .iter()
            .map_err(|error| format!("failed to scan theme manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read theme manifest: {error}"))?;
            let stored = decode_stored_theme_manifest(value.value())?;
            let enabled = plugin_enabled_from_table(&plugin_enablement, &stored.plugin_id)?;
            themes.push(ThemeCatalogItem {
                id: stored.theme.id,
                name: stored.theme.name,
                version: stored.theme.version,
                source: "plugin".to_string(),
                plugin_id: Some(stored.plugin_id),
                enabled,
                tokens: stored.theme.tokens,
            });
        }
        themes.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });

        let requested_theme_id = settings
            .get(ACTIVE_THEME_KEY)
            .map_err(|error| format!("failed to read active theme setting: {error}"))?
            .map(|value| value.value().to_string())
            .unwrap_or_else(|| DEFAULT_THEME_ID.to_string());
        let active_theme_id = if themes
            .iter()
            .any(|theme| theme.id == requested_theme_id && theme.enabled)
        {
            requested_theme_id
        } else {
            DEFAULT_THEME_ID.to_string()
        };
        let accent_override = settings
            .get(ACCENT_OVERRIDE_KEY)
            .map_err(|error| format!("failed to read accent override setting: {error}"))?
            .map(|value| value.value().to_string());

        Ok(AppearanceState {
            active_theme_id,
            accent_override,
            themes,
            plugins,
        })
    }

    pub(super) fn set_theme(&mut self, theme_id: &str) -> Result<AppearanceState, String> {
        let theme_id = theme_id.trim();
        if !self
            .state()?
            .themes
            .iter()
            .any(|theme| theme.id == theme_id && theme.enabled)
        {
            return Err(format!("unknown or disabled theme: {theme_id}"));
        }

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write active theme setting: {error}"))?;
        {
            let mut settings = transaction
                .open_table(SETTINGS_KV)
                .map_err(|error| format!("failed to open appearance settings table: {error}"))?;
            settings
                .insert(ACTIVE_THEME_KEY, theme_id)
                .map_err(|error| format!("failed to store active theme setting: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit active theme setting: {error}"))?;
        self.state()
    }

    pub(super) fn set_accent_override(
        &mut self,
        accent: Option<String>,
    ) -> Result<AppearanceState, String> {
        let accent = accent.and_then(|value| {
            let value = value.trim().to_string();
            (!value.is_empty()).then_some(value)
        });
        if let Some(value) = accent.as_deref() {
            validate_color_token("accentOverride", value)?;
        }

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write accent override setting: {error}"))?;
        {
            let mut settings = transaction
                .open_table(SETTINGS_KV)
                .map_err(|error| format!("failed to open appearance settings table: {error}"))?;
            if let Some(value) = accent.as_deref() {
                settings
                    .insert(ACCENT_OVERRIDE_KEY, value)
                    .map_err(|error| format!("failed to store accent override setting: {error}"))?;
            } else {
                settings
                    .remove(ACCENT_OVERRIDE_KEY)
                    .map_err(|error| format!("failed to clear accent override setting: {error}"))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit accent override setting: {error}"))?;
        self.state()
    }

    pub(super) fn preferences(&self) -> Result<PlayerPreferences, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read player preferences: {error}"))?;
        let settings = transaction
            .open_table(SETTINGS_KV)
            .map_err(|error| format!("failed to open player preferences table: {error}"))?;

        Ok(PlayerPreferences {
            incognito_mode: read_bool_setting(&settings, INCOGNITO_MODE_KEY)?,
            quiet_keyboard_controls: read_bool_setting(&settings, QUIET_KEYBOARD_CONTROLS_KEY)?,
            language_mode: read_language_mode_setting(&settings)?,
        })
    }

    pub(super) fn set_bool_preference(
        &mut self,
        key: &'static str,
        enabled: bool,
    ) -> Result<PlayerPreferences, String> {
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write player preference: {error}"))?;
        {
            let mut settings = transaction
                .open_table(SETTINGS_KV)
                .map_err(|error| format!("failed to open player preferences table: {error}"))?;
            settings
                .insert(key, if enabled { "true" } else { "false" })
                .map_err(|error| format!("failed to store player preference: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit player preference: {error}"))?;
        self.preferences()
    }

    pub(super) fn set_language_mode(&mut self, mode: &str) -> Result<PlayerPreferences, String> {
        let mode = validate_language_mode(mode)?;
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write language preference: {error}"))?;
        {
            let mut settings = transaction
                .open_table(SETTINGS_KV)
                .map_err(|error| format!("failed to open player preferences table: {error}"))?;
            settings
                .insert(LANGUAGE_MODE_KEY, mode)
                .map_err(|error| format!("failed to store language preference: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit language preference: {error}"))?;
        self.preferences()
    }

    #[cfg(test)]
    pub(super) fn import_theme_plugin_json(
        &mut self,
        json: &str,
    ) -> Result<AppearanceState, String> {
        let manifest = parse_theme_plugin_manifest_json(json)?;
        self.store_plugin_manifest(manifest, None)
    }

    pub(super) fn import_plugin_manifest_path(
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

    pub(super) fn import_plugin_directory_path(
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

    pub(super) fn import_plugin_package_path(
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

    pub(super) fn store_plugin_manifest(
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

    pub(super) fn uninstall_plugin(&mut self, plugin_id: &str) -> Result<AppearanceState, String> {
        let plugin_id = plugin_id.trim();
        validate_dotted_identifier("plugin id", plugin_id, true)?;
        let active_theme_id = self.state()?.active_theme_id;
        let install_path = self
            .plugin_install_record(plugin_id)?
            .map(|record| record.install_path);

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to uninstall plugin: {error}"))?;
        {
            let mut plugin_manifests = transaction
                .open_table(PLUGIN_MANIFESTS)
                .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
            if plugin_manifests
                .remove(plugin_id)
                .map_err(|error| format!("failed to remove plugin manifest: {error}"))?
                .is_none()
            {
                return Err(format!("unknown plugin: {plugin_id}"));
            }

            let mut plugin_enablement = transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
            plugin_enablement
                .remove(plugin_id)
                .map_err(|error| format!("failed to remove plugin enablement: {error}"))?;

            let mut plugin_installs = transaction
                .open_table(PLUGIN_INSTALLS)
                .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
            plugin_installs
                .remove(plugin_id)
                .map_err(|error| format!("failed to remove plugin install record: {error}"))?;

            let mut theme_manifests = transaction
                .open_table(THEME_MANIFESTS)
                .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
            let active_theme_belongs_to_plugin =
                theme_belongs_to_plugin(&theme_manifests, &active_theme_id, plugin_id)?;
            let stale_theme_ids = theme_manifests_for_plugin(&theme_manifests, plugin_id)?;
            for theme_id in stale_theme_ids {
                theme_manifests
                    .remove(theme_id.as_str())
                    .map_err(|error| format!("failed to remove plugin theme: {error}"))?;
            }

            let mut plugin_settings = transaction
                .open_table(PLUGIN_SETTINGS)
                .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
            let stale_setting_keys = plugin_setting_keys_for_plugin(&plugin_settings, plugin_id)?;
            for key in stale_setting_keys {
                plugin_settings
                    .remove(key.as_str())
                    .map_err(|error| format!("failed to remove plugin setting: {error}"))?;
            }

            let mut plugin_runtime_storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            let stale_storage_keys =
                plugin_runtime_storage_keys_for_plugin(&plugin_runtime_storage, plugin_id)?;
            for key in stale_storage_keys {
                plugin_runtime_storage
                    .remove(key.as_str())
                    .map_err(|error| {
                        format!("failed to remove plugin runtime storage value: {error}")
                    })?;
            }

            if active_theme_belongs_to_plugin {
                let mut settings = transaction.open_table(SETTINGS_KV).map_err(|error| {
                    format!("failed to open appearance settings table: {error}")
                })?;
                settings
                    .insert(ACTIVE_THEME_KEY, DEFAULT_THEME_ID)
                    .map_err(|error| format!("failed to store active theme fallback: {error}"))?;
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin uninstall: {error}"))?;

        if let Some(install_path) = install_path {
            remove_installed_plugin_directory(&self.plugin_root, &PathBuf::from(install_path))?;
        }
        self.state()
    }

    pub(super) fn plugin_install_record(
        &self,
        plugin_id: &str,
    ) -> Result<Option<StoredPluginInstall>, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin install record: {error}"))?;
        let plugin_installs = transaction
            .open_table(PLUGIN_INSTALLS)
            .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
        plugin_install_from_table(&plugin_installs, plugin_id)
    }

    pub(super) fn plugin_install_directory(&self, plugin_id: &str) -> PathBuf {
        self.plugin_root.join(plugin_id)
    }

    pub(super) fn plugin_staging_directory(&self, plugin_id: &str) -> PathBuf {
        self.plugin_root
            .join(format!(".{plugin_id}.installing-{}", current_time_ms()))
    }

    pub(super) fn plugin_runtime_sources(&self) -> Result<Vec<PluginRuntimeSource>, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin runtime sources: {error}"))?;
        let plugin_manifests = transaction
            .open_table(PLUGIN_MANIFESTS)
            .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
        let plugin_enablement = transaction
            .open_table(PLUGIN_ENABLEMENT)
            .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
        let plugin_installs = transaction
            .open_table(PLUGIN_INSTALLS)
            .map_err(|error| format!("failed to open plugin installs table: {error}"))?;

        let mut runtime_sources = Vec::new();
        for item in plugin_manifests
            .iter()
            .map_err(|error| format!("failed to scan plugin manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read plugin manifest: {error}"))?;
            let manifest = decode_plugin_manifest(value.value())?;
            if manifest.runtime.kind != PluginRuntimeKind::WebviewJs
                || !plugin_enabled_from_table(&plugin_enablement, &manifest.id)?
            {
                continue;
            }

            let install = plugin_install_from_table(&plugin_installs, &manifest.id)?
                .ok_or_else(|| format!("runtime plugin {} is not installed", manifest.id))?;
            let entry = manifest
                .runtime
                .entry
                .as_deref()
                .ok_or_else(|| format!("runtime plugin {} is missing an entry", manifest.id))?;
            let script_path = resolve_plugin_runtime_script_path(&install.install_path, entry)?;
            let metadata = fs::metadata(&script_path)
                .map_err(|error| format!("failed to inspect plugin runtime script: {error}"))?;
            if metadata.len() > MAX_PLUGIN_RUNTIME_SCRIPT_BYTES {
                return Err(format!("plugin runtime script is too large: {entry}"));
            }
            let script = fs::read_to_string(&script_path)
                .map_err(|error| format!("failed to read plugin runtime script: {error}"))?;

            runtime_sources.push(PluginRuntimeSource {
                plugin_id: manifest.id.clone(),
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                entry: entry.to_string(),
                script,
                permissions: plugin_permissions(&manifest),
            });
        }
        runtime_sources.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.plugin_id.cmp(&right.plugin_id))
        });
        Ok(runtime_sources)
    }

    pub(super) fn plugin_view_html(
        &self,
        plugin_id: &str,
        view_id: &str,
    ) -> Result<PluginViewHtml, String> {
        let plugin_id = plugin_id.trim();
        let view_id = view_id.trim();
        validate_dotted_identifier("plugin id", plugin_id, true)?;
        validate_dotted_identifier("plugin view id", view_id, false)?;

        let manifest = self.plugin_manifest(plugin_id)?;
        let view = manifest
            .contributes
            .views
            .iter()
            .find(|view| view.id == view_id)
            .ok_or_else(|| format!("unknown plugin view: {plugin_id}.{view_id}"))?;
        let install = self
            .plugin_install_record(plugin_id)?
            .ok_or_else(|| format!("plugin {plugin_id} is not installed"))?;
        let html_path = resolve_plugin_package_file_path(&install.install_path, &view.entry)?;
        let metadata = fs::metadata(&html_path)
            .map_err(|error| format!("failed to inspect plugin view HTML: {error}"))?;
        if metadata.len() > MAX_PLUGIN_VIEW_HTML_BYTES {
            return Err(format!("plugin view HTML is too large: {}", view.entry));
        }
        let html = fs::read_to_string(&html_path)
            .map_err(|error| format!("failed to read plugin view HTML: {error}"))?;

        Ok(PluginViewHtml {
            plugin_id: manifest.id,
            view_id: view.id.clone(),
            title: view.title.clone(),
            html,
        })
    }

    pub(super) fn set_plugin_enabled(
        &mut self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<AppearanceState, String> {
        let plugin_id = plugin_id.trim();
        let active_theme_id = self.state()?.active_theme_id;
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin enablement: {error}"))?;
        {
            let plugin_manifests = transaction
                .open_table(PLUGIN_MANIFESTS)
                .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
            if plugin_manifests
                .get(plugin_id)
                .map_err(|error| format!("failed to read plugin manifest: {error}"))?
                .is_none()
            {
                return Err(format!("unknown theme plugin: {plugin_id}"));
            }

            let mut plugin_enablement = transaction
                .open_table(PLUGIN_ENABLEMENT)
                .map_err(|error| format!("failed to open plugin enablement table: {error}"))?;
            plugin_enablement
                .insert(plugin_id, if enabled { "true" } else { "false" })
                .map_err(|error| format!("failed to store plugin enablement: {error}"))?;

            if !enabled {
                let theme_manifests = transaction
                    .open_table(THEME_MANIFESTS)
                    .map_err(|error| format!("failed to open theme manifest table: {error}"))?;
                if theme_belongs_to_plugin(&theme_manifests, &active_theme_id, plugin_id)? {
                    let mut settings = transaction.open_table(SETTINGS_KV).map_err(|error| {
                        format!("failed to open appearance settings table: {error}")
                    })?;
                    settings
                        .insert(ACTIVE_THEME_KEY, DEFAULT_THEME_ID)
                        .map_err(|error| {
                            format!("failed to store active theme fallback: {error}")
                        })?;
                }
            }
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin enablement: {error}"))?;
        self.state()
    }

    pub(super) fn set_plugin_setting(
        &mut self,
        plugin_id: &str,
        setting_id: &str,
        value: Value,
    ) -> Result<AppearanceState, String> {
        let plugin_id = plugin_id.trim();
        let setting_id = setting_id.trim();
        let manifest = self.plugin_manifest(plugin_id)?;
        let setting = manifest
            .contributes
            .settings
            .iter()
            .find(|setting| setting.id == setting_id)
            .ok_or_else(|| format!("unknown plugin setting: {plugin_id}.{setting_id}"))?;
        validate_plugin_setting_value(setting, &value)?;
        let encoded = serde_json::to_string(&value)
            .map_err(|error| format!("failed to encode plugin setting value: {error}"))?;
        let key = plugin_setting_key(plugin_id, setting_id);

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin setting: {error}"))?;
        {
            let mut settings = transaction
                .open_table(PLUGIN_SETTINGS)
                .map_err(|error| format!("failed to open plugin settings table: {error}"))?;
            settings
                .insert(key.as_str(), encoded.as_str())
                .map_err(|error| format!("failed to store plugin setting: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin setting: {error}"))?;
        self.state()
    }

    pub(super) fn plugin_runtime_storage_value(
        &self,
        plugin_id: &str,
        key: &str,
    ) -> Result<Option<Value>, String> {
        let plugin_id = plugin_id.trim();
        let key = validate_plugin_runtime_storage_key(key)?;
        self.plugin_manifest(plugin_id)?;

        let storage_key = plugin_runtime_storage_key(plugin_id, key);
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin runtime storage: {error}"))?;
        let storage = transaction
            .open_table(PLUGIN_RUNTIME_STORAGE)
            .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
        storage
            .get(storage_key.as_str())
            .map_err(|error| format!("failed to read plugin runtime storage value: {error}"))?
            .map(|value| decode_plugin_runtime_storage_value(value.value()))
            .transpose()
    }

    pub(super) fn plugin_runtime_storage_values(
        &self,
        plugin_id: &str,
    ) -> Result<HashMap<String, Value>, String> {
        let plugin_id = plugin_id.trim();
        let prefix = plugin_runtime_storage_prefix(plugin_id);
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to list plugin runtime storage: {error}"))?;
        let storage = transaction
            .open_table(PLUGIN_RUNTIME_STORAGE)
            .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
        let mut values = HashMap::new();
        for item in storage
            .iter()
            .map_err(|error| format!("failed to scan plugin runtime storage: {error}"))?
        {
            let (key, value) =
                item.map_err(|error| format!("failed to read plugin runtime storage: {error}"))?;
            if let Some(item_key) = key.value().strip_prefix(&prefix) {
                values.insert(
                    item_key.to_string(),
                    decode_plugin_runtime_storage_value(value.value())?,
                );
            }
        }
        Ok(values)
    }

    pub(super) fn set_plugin_runtime_storage_value(
        &mut self,
        plugin_id: &str,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        let plugin_id = plugin_id.trim();
        let key = validate_plugin_runtime_storage_key(key)?;
        self.plugin_manifest(plugin_id)?;
        let encoded = serde_json::to_string(&value)
            .map_err(|error| format!("failed to encode plugin runtime storage value: {error}"))?;
        if encoded.len() > MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES {
            return Err("plugin runtime storage value is too large".to_string());
        }

        let storage_key = plugin_runtime_storage_key(plugin_id, key);
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to write plugin runtime storage: {error}"))?;
        {
            let mut storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            storage
                .insert(storage_key.as_str(), encoded.as_str())
                .map_err(|error| {
                    format!("failed to store plugin runtime storage value: {error}")
                })?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin runtime storage: {error}"))
    }

    pub(super) fn remove_plugin_runtime_storage_value(
        &mut self,
        plugin_id: &str,
        key: &str,
    ) -> Result<bool, String> {
        let plugin_id = plugin_id.trim();
        let key = validate_plugin_runtime_storage_key(key)?;
        self.plugin_manifest(plugin_id)?;
        let storage_key = plugin_runtime_storage_key(plugin_id, key);
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to remove plugin runtime storage: {error}"))?;
        let removed = {
            let mut storage = transaction
                .open_table(PLUGIN_RUNTIME_STORAGE)
                .map_err(|error| format!("failed to open plugin runtime storage table: {error}"))?;
            storage
                .remove(storage_key.as_str())
                .map_err(|error| format!("failed to remove plugin runtime storage value: {error}"))?
                .is_some()
        };
        transaction
            .commit()
            .map_err(|error| format!("failed to commit plugin runtime storage removal: {error}"))?;
        Ok(removed)
    }

    pub(super) fn plugin_manifest(&self, plugin_id: &str) -> Result<PluginManifest, String> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| format!("failed to read plugin manifest: {error}"))?;
        let plugin_manifests = transaction
            .open_table(PLUGIN_MANIFESTS)
            .map_err(|error| format!("failed to open plugin manifest table: {error}"))?;
        let stored = plugin_manifests
            .get(plugin_id)
            .map_err(|error| format!("failed to read plugin manifest: {error}"))?
            .ok_or_else(|| format!("unknown plugin: {plugin_id}"))?;
        decode_plugin_manifest(stored.value())
    }

    pub(super) fn reset(&mut self) -> Result<AppearanceState, String> {
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to reset appearance settings: {error}"))?;
        {
            let mut settings = transaction
                .open_table(SETTINGS_KV)
                .map_err(|error| format!("failed to open appearance settings table: {error}"))?;
            settings
                .insert(ACTIVE_THEME_KEY, DEFAULT_THEME_ID)
                .map_err(|error| format!("failed to reset active theme setting: {error}"))?;
            settings
                .remove(ACCENT_OVERRIDE_KEY)
                .map_err(|error| format!("failed to reset accent override setting: {error}"))?;
        }
        transaction
            .commit()
            .map_err(|error| format!("failed to commit appearance reset: {error}"))?;
        self.state()
    }
}
