use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Manager, State};

const SETTINGS_KV: TableDefinition<&str, &str> = TableDefinition::new("settings_kv");
const THEME_MANIFESTS: TableDefinition<&str, &str> = TableDefinition::new("theme_manifests");
const PLUGIN_MANIFESTS: TableDefinition<&str, &str> = TableDefinition::new("plugin_manifests");
const PLUGIN_ENABLEMENT: TableDefinition<&str, &str> = TableDefinition::new("plugin_enablement");
const PLUGIN_SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("plugin_settings");
const PLUGIN_INSTALLS: TableDefinition<&str, &str> = TableDefinition::new("plugin_installs");
const ACTIVE_THEME_KEY: &str = "activeThemeId";
const ACCENT_OVERRIDE_KEY: &str = "accentOverride";
const INCOGNITO_MODE_KEY: &str = "incognitoMode";
const QUIET_KEYBOARD_CONTROLS_KEY: &str = "quietKeyboardControls";
const LANGUAGE_MODE_KEY: &str = "languageMode";
const DEFAULT_THEME_ID: &str = "studio-dark";
const PLUGIN_MANIFEST_FILE: &str = "manifest.json";
const PLUGIN_PACKAGE_EXTENSION: &str = "opplugin";
const MAX_PLUGIN_PACKAGE_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PLUGIN_PACKAGE_FILES: usize = 1024;
const MAX_PLUGIN_RUNTIME_SCRIPT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTokens {
    surface: String,
    panel: String,
    panel_strong: String,
    text: String,
    muted: String,
    faint: String,
    accent: String,
    danger: String,
    line: String,
    control: String,
    scrollbar_thumb: String,
    scrollbar_thumb_hover: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ThemeManifest {
    id: String,
    name: String,
    version: String,
    tokens: ThemeTokens,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredThemeManifest {
    plugin_id: String,
    theme: ThemeManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredPluginInstall {
    package_kind: String,
    install_path: String,
    installed_at_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginManifest {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    entry: ThemePluginEntry,
    #[serde(default)]
    runtime: PluginRuntime,
    contributes: PluginContributions,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginContributions {
    #[serde(default)]
    themes: Vec<ThemeManifest>,
    #[serde(default)]
    capabilities: Vec<PluginCapabilityManifest>,
    #[serde(default)]
    settings: Vec<PluginSettingManifest>,
    #[serde(default)]
    actions: Vec<PluginActionManifest>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
enum ThemePluginEntry {
    #[serde(rename = "manifest")]
    Manifest,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginRuntime {
    kind: PluginRuntimeKind,
    entry: Option<String>,
    sandbox: Option<String>,
}

impl Default for PluginRuntime {
    fn default() -> Self {
        Self {
            kind: PluginRuntimeKind::Manifest,
            entry: None,
            sandbox: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
enum PluginRuntimeKind {
    #[serde(rename = "manifest")]
    Manifest,
    #[serde(rename = "webviewJs")]
    WebviewJs,
    #[serde(rename = "wasm")]
    Wasm,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginCapabilityManifest {
    id: String,
    name: String,
    kind: String,
    description: Option<String>,
    #[serde(default)]
    name_i18n: HashMap<String, String>,
    #[serde(default)]
    description_i18n: HashMap<String, String>,
    #[serde(default)]
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginSettingManifest {
    id: String,
    label: String,
    description: Option<String>,
    #[serde(default)]
    label_i18n: HashMap<String, String>,
    #[serde(default)]
    description_i18n: HashMap<String, String>,
    kind: String,
    placement: String,
    default_value: Value,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    #[serde(default)]
    options: Vec<PluginSettingOption>,
    mpv_property: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PluginActionManifest {
    id: String,
    label: String,
    description: Option<String>,
    #[serde(default)]
    label_i18n: HashMap<String, String>,
    #[serde(default)]
    description_i18n: HashMap<String, String>,
    placement: String,
    command: String,
    icon: Option<String>,
    #[serde(default)]
    requires_media: bool,
    #[serde(default = "default_plugin_action_args")]
    args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginSettingOption {
    value: String,
    label: String,
    #[serde(default)]
    label_i18n: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemeCatalogItem {
    id: String,
    name: String,
    version: String,
    source: String,
    plugin_id: Option<String>,
    enabled: bool,
    tokens: ThemeTokens,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePluginSummary {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    enabled: bool,
    package_kind: String,
    install_path: Option<String>,
    installed_at_ms: Option<u64>,
    theme_count: usize,
    runtime: String,
    capability_count: usize,
    setting_count: usize,
    action_count: usize,
    permissions: Vec<String>,
    capabilities: Vec<PluginCapabilitySummary>,
    settings: Vec<PluginSettingSummary>,
    actions: Vec<PluginActionSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilitySummary {
    id: String,
    name: String,
    kind: String,
    description: Option<String>,
    name_i18n: HashMap<String, String>,
    description_i18n: HashMap<String, String>,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginSettingSummary {
    id: String,
    label: String,
    description: Option<String>,
    label_i18n: HashMap<String, String>,
    description_i18n: HashMap<String, String>,
    kind: String,
    placement: String,
    default_value: Value,
    value: Value,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    options: Vec<PluginSettingOption>,
    mpv_property: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginActionSummary {
    id: String,
    label: String,
    description: Option<String>,
    label_i18n: HashMap<String, String>,
    description_i18n: HashMap<String, String>,
    placement: String,
    command: String,
    icon: Option<String>,
    requires_media: bool,
    args: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeSource {
    plugin_id: String,
    name: String,
    version: String,
    entry: String,
    script: String,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    active_theme_id: String,
    accent_override: Option<String>,
    themes: Vec<ThemeCatalogItem>,
    plugins: Vec<ThemePluginSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPreferences {
    incognito_mode: bool,
    quiet_keyboard_controls: bool,
    language_mode: String,
}

pub struct AppearanceStoreState {
    path: PathBuf,
    access: Mutex<()>,
}

struct AppearanceStore {
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

    fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
        let mut directory = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
        directory.push("storage");
        Ok(directory.join("openplayer-settings.redb"))
    }

    fn with_store<T>(
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
    fn open(path: PathBuf) -> Result<Self, String> {
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

    fn initialize(&self) -> Result<(), String> {
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
                .open_table(PLUGIN_INSTALLS)
                .map_err(|error| format!("failed to open plugin installs table: {error}"))?;
        }
        transaction.commit().map_err(|error| {
            format!("failed to commit appearance settings initialization: {error}")
        })
    }

    fn state(&self) -> Result<AppearanceState, String> {
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
            let permissions = plugin_permissions(&manifest);
            let install = plugin_install_from_table(&plugin_installs, &manifest.id)?;
            plugins.push(ThemePluginSummary {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
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

    fn set_theme(&mut self, theme_id: &str) -> Result<AppearanceState, String> {
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

    fn set_accent_override(&mut self, accent: Option<String>) -> Result<AppearanceState, String> {
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

    fn preferences(&self) -> Result<PlayerPreferences, String> {
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

    fn set_bool_preference(
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

    fn set_language_mode(&mut self, mode: &str) -> Result<PlayerPreferences, String> {
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
    fn import_theme_plugin_json(&mut self, json: &str) -> Result<AppearanceState, String> {
        let manifest = parse_theme_plugin_manifest_json(json)?;
        self.store_plugin_manifest(manifest, None)
    }

    fn import_plugin_manifest_path(&mut self, path: &Path) -> Result<AppearanceState, String> {
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

    fn import_plugin_directory_path(&mut self, path: &Path) -> Result<AppearanceState, String> {
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

    fn import_plugin_package_path(&mut self, path: &Path) -> Result<AppearanceState, String> {
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

    fn store_plugin_manifest(
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

    fn uninstall_plugin(&mut self, plugin_id: &str) -> Result<AppearanceState, String> {
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

    fn plugin_install_record(
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

    fn plugin_install_directory(&self, plugin_id: &str) -> PathBuf {
        self.plugin_root.join(plugin_id)
    }

    fn plugin_staging_directory(&self, plugin_id: &str) -> PathBuf {
        self.plugin_root
            .join(format!(".{plugin_id}.installing-{}", current_time_ms()))
    }

    fn plugin_runtime_sources(&self) -> Result<Vec<PluginRuntimeSource>, String> {
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

    fn set_plugin_enabled(
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

    fn set_plugin_setting(
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

    fn plugin_manifest(&self, plugin_id: &str) -> Result<PluginManifest, String> {
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

    fn reset(&mut self) -> Result<AppearanceState, String> {
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

fn built_in_theme_catalog() -> Vec<ThemeCatalogItem> {
    built_in_theme_manifests()
        .into_iter()
        .map(|theme| ThemeCatalogItem {
            id: theme.id,
            name: theme.name,
            version: theme.version,
            source: "builtIn".to_string(),
            plugin_id: None,
            enabled: true,
            tokens: theme.tokens,
        })
        .collect()
}

fn create_database_with_retry(path: &PathBuf, label: &str) -> Result<Database, String> {
    let mut last_error = None;
    for _ in 0..16 {
        match Database::create(path) {
            Ok(database) => return Ok(database),
            Err(error) => {
                last_error = Some(error.to_string());
                thread::sleep(Duration::from_millis(20));
            }
        }
    }

    Err(format!(
        "failed to open {label} database: {}",
        last_error.unwrap_or_else(|| "unknown redb error".to_string())
    ))
}

fn built_in_theme_manifests() -> Vec<ThemeManifest> {
    vec![ThemeManifest {
        id: DEFAULT_THEME_ID.to_string(),
        name: "Studio Dark".to_string(),
        version: "1.0.0".to_string(),
        tokens: ThemeTokens {
            surface: "#050607".to_string(),
            panel: "rgba(8, 10, 12, 0.72)".to_string(),
            panel_strong: "rgba(8, 10, 12, 0.88)".to_string(),
            text: "#ece7dd".to_string(),
            muted: "#b9b0a3".to_string(),
            faint: "#8f867a".to_string(),
            accent: "#caa05d".to_string(),
            danger: "#d78372".to_string(),
            line: "rgba(236, 231, 221, 0.12)".to_string(),
            control: "rgba(18, 21, 25, 0.72)".to_string(),
            scrollbar_thumb: "rgba(236, 231, 221, 0.22)".to_string(),
            scrollbar_thumb_hover: "rgba(202, 160, 93, 0.46)".to_string(),
        },
    }]
}

fn parse_theme_plugin_manifest_json(json: &str) -> Result<PluginManifest, String> {
    let manifest: PluginManifest = serde_json::from_str(json)
        .map_err(|error| format!("invalid plugin manifest JSON: {error}"))?;
    validate_plugin_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_plugin_manifest(manifest: &PluginManifest) -> Result<(), String> {
    validate_non_empty("plugin id", &manifest.id)?;
    validate_non_empty("plugin name", &manifest.name)?;
    validate_non_empty("plugin version", &manifest.version)?;
    validate_dotted_identifier("plugin id", &manifest.id, true)?;
    validate_simple_semver("plugin version", &manifest.version)?;
    if let Some(description) = manifest.description.as_deref() {
        validate_non_empty("plugin description", description)?;
    }
    validate_plugin_runtime(&manifest.runtime)?;
    if manifest.contributes.themes.is_empty()
        && manifest.contributes.capabilities.is_empty()
        && manifest.contributes.settings.is_empty()
        && manifest.contributes.actions.is_empty()
    {
        return Err(
            "plugin must contribute at least one theme, capability, setting, or action".to_string(),
        );
    }

    let mut ids = HashSet::new();
    for theme in &manifest.contributes.themes {
        validate_theme_manifest(theme)?;
        if !ids.insert(theme.id.as_str()) {
            return Err(format!("duplicate theme id: {}", theme.id));
        }
    }

    let mut capability_ids = HashSet::new();
    for capability in &manifest.contributes.capabilities {
        validate_plugin_capability(capability)?;
        if !capability_ids.insert(capability.id.as_str()) {
            return Err(format!("duplicate capability id: {}", capability.id));
        }
    }

    let mut setting_ids = HashSet::new();
    for setting in &manifest.contributes.settings {
        validate_plugin_setting(setting)?;
        if !setting_ids.insert(setting.id.as_str()) {
            return Err(format!("duplicate setting id: {}", setting.id));
        }
    }

    let permissions = plugin_permissions(manifest);
    let mut action_ids = HashSet::new();
    for action in &manifest.contributes.actions {
        validate_plugin_action(action, &permissions)?;
        if !action_ids.insert(action.id.as_str()) {
            return Err(format!("duplicate action id: {}", action.id));
        }
    }

    Ok(())
}

fn validate_plugin_runtime(runtime: &PluginRuntime) -> Result<(), String> {
    match runtime.kind {
        PluginRuntimeKind::Manifest => {
            if let Some(entry) = runtime.entry.as_deref() {
                validate_relative_plugin_entry(entry)?;
            }
            if let Some(sandbox) = runtime.sandbox.as_deref() {
                validate_non_empty("plugin runtime sandbox", sandbox)?;
            }
            Ok(())
        }
        PluginRuntimeKind::WebviewJs => {
            let Some(entry) = runtime.entry.as_deref() else {
                return Err("plugin runtime webviewJs requires an entry".to_string());
            };
            validate_relative_plugin_entry(entry)?;
            if let Some(sandbox) = runtime.sandbox.as_deref()
                && sandbox != "openplayer-worker"
            {
                return Err(
                    "plugin runtime webviewJs requires the openplayer-worker sandbox".to_string(),
                );
            }
            Ok(())
        }
        PluginRuntimeKind::Wasm => Err(format!(
            "plugin runtime {} is not supported yet",
            runtime_kind_label(&runtime.kind)
        )),
    }
}

fn validate_relative_plugin_entry(entry: &str) -> Result<(), String> {
    validate_non_empty("plugin runtime entry", entry)?;
    if entry.contains('\\') || entry.starts_with('/') || entry.contains("..") {
        Err("plugin runtime entry must be a relative package path".to_string())
    } else {
        Ok(())
    }
}

fn validate_plugin_capability(capability: &PluginCapabilityManifest) -> Result<(), String> {
    validate_non_empty("plugin capability id", &capability.id)?;
    validate_non_empty("plugin capability name", &capability.name)?;
    validate_dotted_identifier("plugin capability id", &capability.id, false)?;
    if let Some(description) = capability.description.as_deref() {
        validate_non_empty("plugin capability description", description)?;
    }
    validate_localized_text_map("plugin capability nameI18n", &capability.name_i18n, 128)?;
    validate_localized_text_map(
        "plugin capability descriptionI18n",
        &capability.description_i18n,
        512,
    )?;
    if !is_supported_capability_kind(&capability.kind) {
        return Err(format!(
            "unsupported plugin capability kind: {}",
            capability.kind
        ));
    }
    for permission in &capability.permissions {
        if !is_supported_plugin_permission(permission) {
            return Err(format!("unsupported plugin permission: {permission}"));
        }
    }
    Ok(())
}

fn validate_plugin_setting(setting: &PluginSettingManifest) -> Result<(), String> {
    validate_non_empty("plugin setting id", &setting.id)?;
    validate_non_empty("plugin setting label", &setting.label)?;
    validate_dotted_identifier("plugin setting id", &setting.id, false)?;
    if let Some(description) = setting.description.as_deref() {
        validate_non_empty("plugin setting description", description)?;
    }
    validate_localized_text_map("plugin setting labelI18n", &setting.label_i18n, 128)?;
    validate_localized_text_map(
        "plugin setting descriptionI18n",
        &setting.description_i18n,
        512,
    )?;
    if !is_supported_setting_kind(&setting.kind) {
        return Err(format!("unsupported plugin setting kind: {}", setting.kind));
    }
    if !is_supported_setting_placement(&setting.placement) {
        return Err(format!(
            "unsupported plugin setting placement: {}",
            setting.placement
        ));
    }
    validate_setting_number_bounds(setting)?;
    validate_setting_options(setting)?;
    if let Some(property) = setting.mpv_property.as_deref() {
        validate_plugin_mpv_property(property)?;
        if setting.placement != "subtitleSettings" {
            return Err(format!(
                "mpv property setting {} must use subtitleSettings placement",
                setting.id
            ));
        }
    }
    validate_plugin_setting_value(setting, &setting.default_value)
}

fn validate_plugin_action(
    action: &PluginActionManifest,
    permissions: &[String],
) -> Result<(), String> {
    validate_non_empty("plugin action id", &action.id)?;
    validate_non_empty("plugin action label", &action.label)?;
    validate_dotted_identifier("plugin action id", &action.id, false)?;
    if let Some(description) = action.description.as_deref() {
        validate_non_empty("plugin action description", description)?;
    }
    validate_localized_text_map("plugin action labelI18n", &action.label_i18n, 128)?;
    validate_localized_text_map(
        "plugin action descriptionI18n",
        &action.description_i18n,
        512,
    )?;
    if !is_supported_action_placement(&action.placement) {
        return Err(format!(
            "unsupported plugin action placement: {}",
            action.placement
        ));
    }
    if !is_supported_plugin_action_command(&action.command) {
        return Err(format!(
            "unsupported plugin action command: {}",
            action.command
        ));
    }
    if let Some(icon) = action.icon.as_deref()
        && !is_supported_plugin_action_icon(icon)
    {
        return Err(format!("unsupported plugin action icon: {icon}"));
    }
    if let Some(permission) = plugin_action_required_permission(&action.command)
        && !permissions.iter().any(|item| item == permission)
    {
        return Err(format!(
            "plugin action {} requires permission {}",
            action.id, permission
        ));
    }
    validate_plugin_action_args(action)?;
    Ok(())
}

fn validate_setting_number_bounds(setting: &PluginSettingManifest) -> Result<(), String> {
    if let Some(min) = setting.min
        && !min.is_finite()
    {
        return Err(format!("plugin setting {} min is invalid", setting.id));
    }
    if let Some(max) = setting.max
        && !max.is_finite()
    {
        return Err(format!("plugin setting {} max is invalid", setting.id));
    }
    if let Some(step) = setting.step
        && (!step.is_finite() || step <= 0.0)
    {
        return Err(format!("plugin setting {} step is invalid", setting.id));
    }
    if let (Some(min), Some(max)) = (setting.min, setting.max)
        && min > max
    {
        return Err(format!("plugin setting {} min exceeds max", setting.id));
    }
    Ok(())
}

fn validate_setting_options(setting: &PluginSettingManifest) -> Result<(), String> {
    if setting.kind != "select" {
        if !setting.options.is_empty() {
            return Err(format!(
                "plugin setting {} options are only valid for select settings",
                setting.id
            ));
        }
        return Ok(());
    }
    if setting.options.is_empty() {
        return Err(format!(
            "plugin setting {} select options cannot be empty",
            setting.id
        ));
    }
    let mut values = HashSet::new();
    for option in &setting.options {
        validate_non_empty("plugin setting option value", &option.value)?;
        validate_non_empty("plugin setting option label", &option.label)?;
        validate_localized_text_map("plugin setting option labelI18n", &option.label_i18n, 128)?;
        if !values.insert(option.value.as_str()) {
            return Err(format!(
                "duplicate option value for plugin setting {}: {}",
                setting.id, option.value
            ));
        }
    }
    Ok(())
}

fn validate_plugin_setting_value(
    setting: &PluginSettingManifest,
    value: &Value,
) -> Result<(), String> {
    match setting.kind.as_str() {
        "boolean" => {
            if value.as_bool().is_some() {
                Ok(())
            } else {
                Err(format!("plugin setting {} expects a boolean", setting.id))
            }
        }
        "number" => {
            let Some(number) = value.as_f64().filter(|value| value.is_finite()) else {
                return Err(format!("plugin setting {} expects a number", setting.id));
            };
            if let Some(min) = setting.min
                && number < min
            {
                return Err(format!("plugin setting {} is below minimum", setting.id));
            }
            if let Some(max) = setting.max
                && number > max
            {
                return Err(format!("plugin setting {} is above maximum", setting.id));
            }
            Ok(())
        }
        "text" => {
            let Some(text) = value.as_str() else {
                return Err(format!("plugin setting {} expects text", setting.id));
            };
            if text.len() > 512 {
                return Err(format!("plugin setting {} text is too long", setting.id));
            }
            Ok(())
        }
        "select" => {
            let Some(selected) = value.as_str() else {
                return Err(format!("plugin setting {} expects a selection", setting.id));
            };
            if setting
                .options
                .iter()
                .any(|option| option.value == selected)
            {
                Ok(())
            } else {
                Err(format!(
                    "plugin setting {} has an unknown option",
                    setting.id
                ))
            }
        }
        "color" => {
            let Some(color) = value.as_str() else {
                return Err(format!("plugin setting {} expects a color", setting.id));
            };
            validate_color_token(&setting.id, color)
        }
        _ => Err(format!("unsupported plugin setting kind: {}", setting.kind)),
    }
}

fn default_plugin_action_args() -> Value {
    serde_json::json!({})
}

fn validate_plugin_action_args(action: &PluginActionManifest) -> Result<(), String> {
    let Some(args) = action.args.as_object() else {
        return Err(format!(
            "plugin action {} args must be an object",
            action.id
        ));
    };

    match action.command.as_str() {
        "player.captureScreenshot" => {
            for key in args.keys() {
                if key != "openFolder" {
                    return Err(format!(
                        "plugin action {} has unknown argument: {key}",
                        action.id
                    ));
                }
            }
            if let Some(open_folder) = args.get("openFolder")
                && !open_folder.is_boolean()
            {
                return Err(format!(
                    "plugin action {} openFolder argument must be boolean",
                    action.id
                ));
            }
            Ok(())
        }
        "player.openStream" => {
            let Some(url) = args.get("url").and_then(Value::as_str) else {
                return Err(format!("plugin action {} requires a stream url", action.id));
            };
            validate_plugin_stream_url(url)?;
            for key in args.keys() {
                if key != "url" && key != "name" {
                    return Err(format!(
                        "plugin action {} has unknown argument: {key}",
                        action.id
                    ));
                }
            }
            if let Some(name) = args.get("name").and_then(Value::as_str)
                && (name.trim().is_empty() || name.len() > 128)
            {
                return Err(format!(
                    "plugin action {} stream name is invalid",
                    action.id
                ));
            }
            Ok(())
        }
        _ => {
            if args.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "plugin action {} does not accept arguments",
                    action.id
                ))
            }
        }
    }
}

fn validate_plugin_stream_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    validate_non_empty("plugin stream url", trimmed)?;
    if trimmed.len() > 2048 || trimmed.chars().any(char::is_whitespace) {
        return Err("plugin stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err("plugin stream url must include a protocol".to_string());
    };
    if rest.trim_matches('/').is_empty() {
        return Err("plugin stream url must include a host or path".to_string());
    }
    if is_supported_plugin_stream_scheme(&scheme.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(format!("unsupported plugin stream protocol: {scheme}"))
    }
}

fn is_supported_capability_kind(kind: &str) -> bool {
    matches!(
        kind,
        "subtitleStyle" | "capture" | "streamSource" | "aiTranscription" | "aiTranslation"
    )
}

fn is_supported_plugin_permission(permission: &str) -> bool {
    matches!(
        permission,
        "mpv.subtitleStyle"
            | "mpv.capture"
            | "media.openStream"
            | "network.request"
            | "ai.transcribe"
            | "ai.translate"
    )
}

fn is_supported_setting_kind(kind: &str) -> bool {
    matches!(kind, "boolean" | "number" | "text" | "select" | "color")
}

fn is_supported_setting_placement(placement: &str) -> bool {
    matches!(
        placement,
        "pluginSettings"
            | "subtitleSettings"
            | "captureSettings"
            | "streamSettings"
            | "controls.left"
            | "controls.center"
            | "controls.right"
            | "contextMenu"
            | "overlay.status"
            | "playlist.actions"
    )
}

fn is_supported_action_placement(placement: &str) -> bool {
    matches!(
        placement,
        "controls.left"
            | "controls.center"
            | "controls.right"
            | "contextMenu"
            | "overlay.status"
            | "playlist.actions"
    )
}

fn is_supported_plugin_action_command(command: &str) -> bool {
    matches!(
        command,
        "player.openMedia"
            | "player.openStream"
            | "player.captureScreenshot"
            | "player.togglePlayback"
            | "player.stop"
            | "player.restart"
            | "player.togglePlaylist"
            | "player.toggleTracks"
            | "player.toggleLoop"
            | "player.toggleSpeed"
            | "window.toggleFullscreen"
            | "window.toggleAlwaysOnTop"
            | "app.openSettings"
    )
}

fn plugin_action_required_permission(command: &str) -> Option<&'static str> {
    match command {
        "player.captureScreenshot" => Some("mpv.capture"),
        "player.openStream" => Some("media.openStream"),
        _ => None,
    }
}

fn is_supported_plugin_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

fn is_supported_plugin_action_icon(icon: &str) -> bool {
    matches!(
        icon,
        "folder"
            | "folderAdd"
            | "play"
            | "pause"
            | "stop"
            | "restart"
            | "list"
            | "tracks"
            | "settings"
            | "fullscreen"
            | "pin"
            | "plugin"
            | "camera"
            | "stream"
            | "info"
    )
}

fn validate_plugin_mpv_property(property: &str) -> Result<(), String> {
    if is_allowed_plugin_mpv_property(property) {
        Ok(())
    } else {
        Err(format!("unsupported plugin mpv property: {property}"))
    }
}

fn is_allowed_plugin_mpv_property(property: &str) -> bool {
    matches!(
        property,
        "sub-font"
            | "sub-font-size"
            | "sub-scale"
            | "sub-pos"
            | "sub-color"
            | "sub-spacing"
            | "sub-outline-size"
            | "sub-border-size"
            | "sub-shadow-offset"
    )
}

fn validate_localized_text_map(
    label: &str,
    values: &HashMap<String, String>,
    max_len: usize,
) -> Result<(), String> {
    for (locale, text) in values {
        validate_locale_key(label, locale)?;
        validate_non_empty(label, text)?;
        if text.len() > max_len {
            return Err(format!("{label} value is too long"));
        }
    }
    Ok(())
}

fn validate_locale_key(label: &str, locale: &str) -> Result<(), String> {
    if locale.is_empty()
        || locale.len() > 16
        || !locale
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-' || char == '_')
    {
        return Err(format!("{label} contains an invalid locale key: {locale}"));
    }
    Ok(())
}

fn validate_theme_manifest(theme: &ThemeManifest) -> Result<(), String> {
    validate_non_empty("theme id", &theme.id)?;
    validate_non_empty("theme name", &theme.name)?;
    validate_non_empty("theme version", &theme.version)?;
    validate_dotted_identifier("theme id", &theme.id, false)?;
    validate_simple_semver("theme version", &theme.version)?;
    validate_theme_tokens(&theme.tokens)
}

fn validate_theme_tokens(tokens: &ThemeTokens) -> Result<(), String> {
    validate_color_token("surface", &tokens.surface)?;
    validate_color_token("panel", &tokens.panel)?;
    validate_color_token("panelStrong", &tokens.panel_strong)?;
    validate_color_token("text", &tokens.text)?;
    validate_color_token("muted", &tokens.muted)?;
    validate_color_token("faint", &tokens.faint)?;
    validate_color_token("accent", &tokens.accent)?;
    validate_color_token("danger", &tokens.danger)?;
    validate_color_token("line", &tokens.line)?;
    validate_color_token("control", &tokens.control)?;
    validate_color_token("scrollbarThumb", &tokens.scrollbar_thumb)?;
    validate_color_token("scrollbarThumbHover", &tokens.scrollbar_thumb_hover)
}

fn validate_non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} cannot be empty"))
    } else {
        Ok(())
    }
}

fn validate_dotted_identifier(label: &str, value: &str, require_dot: bool) -> Result<(), String> {
    if require_dot && !value.contains('.') {
        return Err(format!("{label} must use a dotted identifier"));
    }
    if value.split('.').all(is_identifier_segment) {
        Ok(())
    } else {
        Err(format!("{label} is invalid: {value}"))
    }
}

fn is_identifier_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|char| char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-')
}

fn validate_simple_semver(label: &str, value: &str) -> Result<(), String> {
    let parts: Vec<&str> = value.split('.').collect();
    if parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|char| char.is_ascii_digit()))
    {
        Ok(())
    } else {
        Err(format!("{label} must use major.minor.patch"))
    }
}

fn validate_color_token(token: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if is_hex_color(value) || is_rgba_color(value) {
        Ok(())
    } else {
        Err(format!("{token} color is invalid: {value}"))
    }
}

fn is_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|char| char.is_ascii_hexdigit())
}

fn is_rgba_color(value: &str) -> bool {
    let Some(inner) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return false;
    }

    let rgb_ok = parts[..3]
        .iter()
        .all(|part| part.parse::<u16>().is_ok_and(|value| value <= 255));
    let alpha_ok = parts[3]
        .parse::<f64>()
        .is_ok_and(|value| (0.0..=1.0).contains(&value));
    rgb_ok && alpha_ok
}

fn plugin_enabled_from_table<T>(table: &T, plugin_id: &str) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    Ok(table
        .get(plugin_id)
        .map_err(|error| format!("failed to read plugin enablement: {error}"))?
        .map(|value| value.value() != "false")
        .unwrap_or(true))
}

fn plugin_setting_summaries<T>(
    table: &T,
    manifest: &PluginManifest,
) -> Result<Vec<PluginSettingSummary>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let mut settings = Vec::new();
    for setting in &manifest.contributes.settings {
        if let Some(property) = setting.mpv_property.as_deref()
            && !is_allowed_plugin_mpv_property(property)
        {
            continue;
        }
        let key = plugin_setting_key(&manifest.id, &setting.id);
        let stored_value = table
            .get(key.as_str())
            .map_err(|error| format!("failed to read plugin setting: {error}"))?
            .map(|stored| decode_plugin_setting_value(stored.value()))
            .transpose()?;
        let value = stored_value
            .filter(|value| validate_plugin_setting_value(setting, value).is_ok())
            .unwrap_or_else(|| setting.default_value.clone());
        settings.push(PluginSettingSummary {
            id: setting.id.clone(),
            label: setting.label.clone(),
            description: setting.description.clone(),
            label_i18n: setting.label_i18n.clone(),
            description_i18n: setting.description_i18n.clone(),
            kind: setting.kind.clone(),
            placement: setting.placement.clone(),
            default_value: setting.default_value.clone(),
            value,
            min: setting.min,
            max: setting.max,
            step: setting.step,
            options: setting.options.clone(),
            mpv_property: setting.mpv_property.clone(),
        });
    }
    Ok(settings)
}

fn plugin_capability_summaries(manifest: &PluginManifest) -> Vec<PluginCapabilitySummary> {
    manifest
        .contributes
        .capabilities
        .iter()
        .map(|capability| PluginCapabilitySummary {
            id: capability.id.clone(),
            name: capability.name.clone(),
            kind: capability.kind.clone(),
            description: capability.description.clone(),
            name_i18n: capability.name_i18n.clone(),
            description_i18n: capability.description_i18n.clone(),
            permissions: capability.permissions.clone(),
        })
        .collect()
}

fn plugin_action_summaries(manifest: &PluginManifest) -> Vec<PluginActionSummary> {
    manifest
        .contributes
        .actions
        .iter()
        .map(|action| PluginActionSummary {
            id: action.id.clone(),
            label: action.label.clone(),
            description: action.description.clone(),
            label_i18n: action.label_i18n.clone(),
            description_i18n: action.description_i18n.clone(),
            placement: action.placement.clone(),
            command: action.command.clone(),
            icon: action.icon.clone(),
            requires_media: action.requires_media,
            args: action.args.clone(),
        })
        .collect()
}

fn plugin_permissions(manifest: &PluginManifest) -> Vec<String> {
    let mut permissions: Vec<String> = manifest
        .contributes
        .capabilities
        .iter()
        .flat_map(|capability| capability.permissions.iter().cloned())
        .collect();
    permissions.sort();
    permissions.dedup();
    permissions
}

fn plugin_install_from_table<T>(
    table: &T,
    plugin_id: &str,
) -> Result<Option<StoredPluginInstall>, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    table
        .get(plugin_id)
        .map_err(|error| format!("failed to read plugin install record: {error}"))?
        .map(|value| decode_plugin_install_record(value.value()))
        .transpose()
}

fn plugin_setting_key(plugin_id: &str, setting_id: &str) -> String {
    format!("{plugin_id}::{setting_id}")
}

fn plugin_setting_keys_for_plugin(
    table: &redb::Table<'_, &str, &str>,
    plugin_id: &str,
) -> Result<Vec<String>, String> {
    let prefix = format!("{plugin_id}::");
    let mut keys = Vec::new();
    for item in table
        .iter()
        .map_err(|error| format!("failed to scan plugin settings: {error}"))?
    {
        let (key, _) = item.map_err(|error| format!("failed to read plugin setting: {error}"))?;
        if key.value().starts_with(&prefix) {
            keys.push(key.value().to_string());
        }
    }
    Ok(keys)
}

fn decode_plugin_setting_value(value: &str) -> Result<Value, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode plugin setting: {error}"))
}

fn runtime_kind_label(kind: &PluginRuntimeKind) -> &'static str {
    match kind {
        PluginRuntimeKind::Manifest => "manifest",
        PluginRuntimeKind::WebviewJs => "webviewJs",
        PluginRuntimeKind::Wasm => "wasm",
    }
}

fn read_bool_setting<T>(table: &T, key: &str) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    Ok(table
        .get(key)
        .map_err(|error| format!("failed to read boolean setting {key}: {error}"))?
        .map(|value| value.value() == "true")
        .unwrap_or(false))
}

fn read_language_mode_setting<T>(table: &T) -> Result<String, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let Some(value) = table
        .get(LANGUAGE_MODE_KEY)
        .map_err(|error| format!("failed to read language preference: {error}"))?
    else {
        return Ok("system".to_string());
    };

    validate_language_mode(value.value()).map(ToOwned::to_owned)
}

fn validate_language_mode(mode: &str) -> Result<&'static str, String> {
    match mode {
        "system" => Ok("system"),
        "en-US" => Ok("en-US"),
        "zh-CN" => Ok("zh-CN"),
        _ => Err("invalid language mode".to_string()),
    }
}

fn theme_manifests_for_plugin(
    table: &redb::Table<'_, &str, &str>,
    plugin_id: &str,
) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for item in table
        .iter()
        .map_err(|error| format!("failed to scan theme manifests: {error}"))?
    {
        let (id, value) =
            item.map_err(|error| format!("failed to read theme manifest: {error}"))?;
        let stored = decode_stored_theme_manifest(value.value())?;
        if stored.plugin_id == plugin_id {
            ids.push(id.value().to_string());
        }
    }
    Ok(ids)
}

fn theme_belongs_to_plugin<T>(table: &T, theme_id: &str, plugin_id: &str) -> Result<bool, String>
where
    T: ReadableTable<&'static str, &'static str>,
{
    let Some(stored) = table
        .get(theme_id)
        .map_err(|error| format!("failed to read active theme manifest: {error}"))?
    else {
        return Ok(false);
    };
    Ok(decode_stored_theme_manifest(stored.value())?.plugin_id == plugin_id)
}

fn decode_plugin_manifest(value: &str) -> Result<PluginManifest, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin manifest: {error}"))
}

fn decode_plugin_install_record(value: &str) -> Result<StoredPluginInstall, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin install record: {error}"))
}

fn decode_stored_theme_manifest(value: &str) -> Result<StoredThemeManifest, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode theme manifest: {error}"))
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn replace_directory_with_writer(
    target: &Path,
    staging: &Path,
    write: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create plugin install root: {error}"))?;
    }
    if staging.exists() {
        fs::remove_dir_all(staging)
            .map_err(|error| format!("failed to clear stale plugin staging directory: {error}"))?;
    }
    fs::create_dir_all(staging)
        .map_err(|error| format!("failed to create plugin staging directory: {error}"))?;

    if let Err(error) = write(staging) {
        let _ = fs::remove_dir_all(staging);
        return Err(error);
    }

    if target.exists() {
        fs::remove_dir_all(target)
            .map_err(|error| format!("failed to replace installed plugin directory: {error}"))?;
    }
    fs::rename(staging, target)
        .map_err(|error| format!("failed to finalize plugin installation: {error}"))
}

fn copy_directory_contents(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create plugin install directory: {error}"))?;
    for entry in
        fs::read_dir(source).map_err(|error| format!("failed to read plugin directory: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read plugin directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect plugin directory entry: {error}"))?;
        if file_type.is_symlink() {
            return Err("plugin directories cannot contain symlinks".to_string());
        }

        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory_contents(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination)
                .map_err(|error| format!("failed to copy plugin file: {error}"))?;
        }
    }
    Ok(())
}

fn read_manifest_from_plugin_package(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open plugin package: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("invalid plugin package: {error}"))?;
    let mut manifest = archive
        .by_name(PLUGIN_MANIFEST_FILE)
        .map_err(|_| "plugin package must contain manifest.json at its root".to_string())?;
    if manifest.size() > 1024 * 1024 {
        return Err("plugin manifest is too large".to_string());
    }
    let mut json = String::new();
    manifest
        .read_to_string(&mut json)
        .map_err(|error| format!("failed to read plugin manifest from package: {error}"))?;
    Ok(json)
}

fn extract_plugin_package(path: &Path, target: &Path) -> Result<(), String> {
    let file =
        File::open(path).map_err(|error| format!("failed to open plugin package: {error}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("invalid plugin package: {error}"))?;
    if archive.len() > MAX_PLUGIN_PACKAGE_FILES {
        return Err("plugin package contains too many files".to_string());
    }

    let mut total_uncompressed_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("failed to read plugin package entry: {error}"))?;
        if entry.is_symlink() {
            return Err("plugin packages cannot contain symlinks".to_string());
        }
        total_uncompressed_size = total_uncompressed_size.saturating_add(entry.size());
        if total_uncompressed_size > MAX_PLUGIN_PACKAGE_UNCOMPRESSED_BYTES {
            return Err("plugin package is too large".to_string());
        }

        let Some(relative_path) = entry.enclosed_name() else {
            return Err("plugin package contains an unsafe path".to_string());
        };
        if relative_path.as_os_str().is_empty() {
            continue;
        }
        let output_path = target.join(relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|error| format!("failed to create plugin package directory: {error}"))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create plugin package directory: {error}"))?;
        }
        let mut output = File::create(&output_path)
            .map_err(|error| format!("failed to extract plugin package file: {error}"))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("failed to write plugin package file: {error}"))?;
    }

    if !target.join(PLUGIN_MANIFEST_FILE).is_file() {
        return Err("plugin package must contain manifest.json at its root".to_string());
    }
    Ok(())
}

fn remove_installed_plugin_directory(
    plugin_root: &Path,
    install_path: &Path,
) -> Result<(), String> {
    if !install_path.exists() {
        return Ok(());
    }
    let root = fs::canonicalize(plugin_root)
        .map_err(|error| format!("failed to resolve plugin root: {error}"))?;
    let target = fs::canonicalize(install_path)
        .map_err(|error| format!("failed to resolve plugin install directory: {error}"))?;
    if !target.starts_with(root) {
        return Err("plugin install path is outside the managed plugin directory".to_string());
    }
    fs::remove_dir_all(target)
        .map_err(|error| format!("failed to remove installed plugin files: {error}"))
}

fn resolve_plugin_runtime_script_path(install_path: &str, entry: &str) -> Result<PathBuf, String> {
    validate_relative_plugin_entry(entry)?;
    let install_root = PathBuf::from(install_path);
    let root = fs::canonicalize(&install_root)
        .map_err(|error| format!("failed to resolve plugin install path: {error}"))?;
    let candidate = install_root.join(entry);
    let script = fs::canonicalize(&candidate)
        .map_err(|error| format!("failed to resolve plugin runtime script: {error}"))?;
    if !script.starts_with(&root) {
        return Err("plugin runtime script is outside the installed plugin directory".to_string());
    }
    if !script.is_file() {
        return Err(format!("plugin runtime script is not a file: {entry}"));
    }
    Ok(script)
}

#[tauri::command]
pub fn appearance_state(state: State<'_, AppearanceStoreState>) -> Result<AppearanceState, String> {
    state.with_store(|store| store.state())
}

#[tauri::command]
pub fn appearance_set_theme(
    state: State<'_, AppearanceStoreState>,
    theme_id: String,
) -> Result<AppearanceState, String> {
    state.with_store(|store| store.set_theme(&theme_id))
}

#[tauri::command]
pub fn appearance_set_accent_override(
    state: State<'_, AppearanceStoreState>,
    accent: Option<String>,
) -> Result<AppearanceState, String> {
    state.with_store(|store| store.set_accent_override(accent))
}

#[tauri::command]
pub fn preferences_state(
    state: State<'_, AppearanceStoreState>,
) -> Result<PlayerPreferences, String> {
    state.with_store(|store| store.preferences())
}

#[tauri::command]
pub fn preferences_set_incognito_mode(
    state: State<'_, AppearanceStoreState>,
    enabled: bool,
) -> Result<PlayerPreferences, String> {
    state.with_store(|store| store.set_bool_preference(INCOGNITO_MODE_KEY, enabled))
}

#[tauri::command]
pub fn preferences_set_quiet_keyboard_controls(
    state: State<'_, AppearanceStoreState>,
    enabled: bool,
) -> Result<PlayerPreferences, String> {
    state.with_store(|store| store.set_bool_preference(QUIET_KEYBOARD_CONTROLS_KEY, enabled))
}

#[tauri::command]
pub fn preferences_set_language_mode(
    state: State<'_, AppearanceStoreState>,
    mode: String,
) -> Result<PlayerPreferences, String> {
    state.with_store(|store| store.set_language_mode(&mode))
}

#[tauri::command]
pub fn appearance_import_plugin_manifest(
    state: State<'_, AppearanceStoreState>,
    path: String,
) -> Result<AppearanceState, String> {
    let path = PathBuf::from(path.trim());
    state.with_store(|store| store.import_plugin_manifest_path(&path))
}

#[tauri::command]
pub fn appearance_import_plugin_package(
    state: State<'_, AppearanceStoreState>,
    path: String,
) -> Result<AppearanceState, String> {
    let path = PathBuf::from(path.trim());
    state.with_store(|store| store.import_plugin_package_path(&path))
}

#[tauri::command]
pub fn appearance_import_plugin_directory(
    state: State<'_, AppearanceStoreState>,
    path: String,
) -> Result<AppearanceState, String> {
    let path = PathBuf::from(path.trim());
    state.with_store(|store| store.import_plugin_directory_path(&path))
}

#[tauri::command]
pub fn appearance_import_theme_plugin(
    state: State<'_, AppearanceStoreState>,
    path: String,
) -> Result<AppearanceState, String> {
    appearance_import_plugin_manifest(state, path)
}

#[tauri::command]
pub fn appearance_plugin_runtime_sources(
    state: State<'_, AppearanceStoreState>,
) -> Result<Vec<PluginRuntimeSource>, String> {
    state.with_store(|store| store.plugin_runtime_sources())
}

#[tauri::command]
pub fn appearance_uninstall_plugin(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
) -> Result<AppearanceState, String> {
    state.with_store(|store| store.uninstall_plugin(&plugin_id))
}

#[tauri::command]
pub fn appearance_set_plugin_enabled(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
    enabled: bool,
) -> Result<AppearanceState, String> {
    state.with_store(|store| store.set_plugin_enabled(&plugin_id, enabled))
}

#[tauri::command]
pub fn appearance_set_plugin_setting(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
    setting_id: String,
    value: Value,
) -> Result<AppearanceState, String> {
    state.with_store(|store| store.set_plugin_setting(&plugin_id, &setting_id, value))
}

#[tauri::command]
pub fn appearance_reset(state: State<'_, AppearanceStoreState>) -> Result<AppearanceState, String> {
    state.with_store(|store| store.reset())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_STORE_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_store() -> (AppearanceStore, PathBuf) {
        let counter = TEMP_STORE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "openplayer-appearance-{}-{}-{}",
            std::process::id(),
            counter,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("temp appearance directory should be created");
        let store = AppearanceStore::open(directory.join("settings.redb"))
            .expect("appearance store should open");
        (store, directory)
    }

    fn ocean_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.theme.ocean",
          "name": "Ocean Theme Pack",
          "version": "1.0.0",
          "description": "Ocean themes for OpenPlayer.",
          "entry": "manifest",
          "contributes": {
            "themes": [
              {
                "id": "dev.openplayer.theme.ocean.dark",
                "name": "Ocean Dark",
                "version": "1.0.0",
                "tokens": {
                  "surface": "#050607",
                  "panel": "rgba(8, 10, 12, 0.72)",
                  "panelStrong": "rgba(8, 10, 12, 0.88)",
                  "text": "#ece7dd",
                  "muted": "#b9b0a3",
                  "faint": "#8f867a",
                  "accent": "#62c7b7",
                  "danger": "#d78372",
                  "line": "rgba(236, 231, 221, 0.12)",
                  "control": "rgba(18, 21, 25, 0.72)",
                  "scrollbarThumb": "rgba(236, 231, 221, 0.22)",
                  "scrollbarThumbHover": "rgba(98, 199, 183, 0.46)"
                }
              }
            ]
          }
        }"##
    }

    fn subtitle_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.subtitle.styler",
          "name": "Subtitle Styler",
          "version": "1.0.0",
          "description": "Subtitle typography controls for OpenPlayer.",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "subtitle-style",
                "name": "Subtitle Styling",
                "kind": "subtitleStyle",
                "description": "Controls allowed subtitle mpv properties.",
                "permissions": ["mpv.subtitleStyle"]
              }
            ],
            "settings": [
              {
                "id": "font-size",
                "label": "Font Size",
                "description": "Subtitle font size in screen-scaled points.",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 42,
                "min": 12,
                "max": 96,
                "step": 1,
                "mpvProperty": "sub-font-size"
              },
              {
                "id": "font-family",
                "label": "Font Family",
                "kind": "text",
                "placement": "subtitleSettings",
                "defaultValue": "sans-serif",
                "mpvProperty": "sub-font"
              }
            ]
          }
        }"##
    }

    fn extended_subtitle_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.subtitle.typography",
          "name": "Subtitle Typography",
          "version": "1.0.0",
          "description": "Extended subtitle typography controls for OpenPlayer.",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "subtitle-style",
                "name": "Subtitle Styling",
                "kind": "subtitleStyle",
                "description": "Controls allowed subtitle mpv properties.",
                "permissions": ["mpv.subtitleStyle"]
              }
            ],
            "settings": [
              {
                "id": "letter-spacing",
                "label": "Letter Spacing",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 0,
                "min": -10,
                "max": 10,
                "step": 1,
                "mpvProperty": "sub-spacing"
              },
              {
                "id": "outline",
                "label": "Outline",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 3,
                "min": 0,
                "max": 12,
                "step": 0.5,
                "mpvProperty": "sub-outline-size"
              },
              {
                "id": "shadow",
                "label": "Shadow",
                "kind": "number",
                "placement": "subtitleSettings",
                "defaultValue": 1,
                "min": 0,
                "max": 12,
                "step": 0.5,
                "mpvProperty": "sub-shadow-offset"
              }
            ]
          }
        }"##
    }

    fn webview_runtime_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.runtime.worker",
          "name": "Worker Runtime",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "webviewJs",
            "entry": "dist/plugin.js",
            "sandbox": "openplayer-worker"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "capture",
                "name": "Capture",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              }
            ],
            "actions": [
              {
                "id": "runtime-info",
                "label": "Runtime Info",
                "placement": "contextMenu",
                "command": "app.openSettings",
                "icon": "plugin"
              }
            ]
          }
        }"##
    }

    fn wasm_runtime_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.runtime.wasm",
          "name": "Wasm Runtime",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "wasm",
            "entry": "plugin.wasm",
            "sandbox": "openplayer-wasm"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "capture",
                "name": "Capture",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              }
            ]
          }
        }"##
    }

    fn action_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.quick.actions",
          "name": "Quick Actions",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "quick-controls",
                "name": "Quick Controls",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              }
            ],
            "actions": [
              {
                "id": "toggle-tracks",
                "label": "Tracks",
                "description": "Open the track and subtitle panel.",
                "placement": "controls.right",
                "command": "player.toggleTracks",
                "icon": "tracks",
                "requiresMedia": true
              },
              {
                "id": "open-settings",
                "label": "Settings",
                "placement": "contextMenu",
                "command": "app.openSettings",
                "icon": "settings"
              }
            ]
          }
        }"##
    }

    fn capability_action_plugin_json() -> &'static str {
        r##"{
          "id": "dev.openplayer.capability.actions",
          "name": "Capability Actions",
          "version": "1.0.0",
          "entry": "manifest",
          "runtime": {
            "kind": "manifest"
          },
          "contributes": {
            "capabilities": [
              {
                "id": "capture",
                "name": "Capture",
                "kind": "capture",
                "permissions": ["mpv.capture"]
              },
              {
                "id": "streams",
                "name": "Streams",
                "kind": "streamSource",
                "permissions": ["media.openStream"]
              }
            ],
            "actions": [
              {
                "id": "screenshot",
                "label": "Screenshot",
                "placement": "controls.right",
                "command": "player.captureScreenshot",
                "icon": "camera",
                "requiresMedia": true,
                "args": {
                  "openFolder": true
                }
              },
              {
                "id": "open-stream",
                "label": "Open Stream",
                "placement": "playlist.actions",
                "command": "player.openStream",
                "icon": "stream",
                "args": {
                  "url": "https://example.com/live.m3u8",
                  "name": "Live Stream"
                }
              }
            ]
          }
        }"##
    }

    fn write_opplugin_package(path: &Path, manifest_json: &str) {
        let file = File::create(path).expect("plugin package should be created");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file(PLUGIN_MANIFEST_FILE, options)
            .expect("plugin package manifest entry should start");
        writer
            .write_all(manifest_json.as_bytes())
            .expect("plugin manifest should be written to package");
        writer
            .add_directory("assets/", options)
            .expect("plugin package asset directory should be added");
        writer
            .start_file("assets/readme.txt", options)
            .expect("plugin package asset entry should start");
        writer
            .write_all(b"package asset")
            .expect("plugin package asset should be written");
        writer.finish().expect("plugin package should finalize");
    }

    #[test]
    fn redb_store_persists_theme_and_accent_override() {
        let (mut store, directory) = temp_store();
        store
            .set_accent_override(Some("#78d5b3".to_string()))
            .expect("valid accent should be persisted");
        drop(store);

        let store = AppearanceStore::open(directory.join("settings.redb"))
            .expect("appearance store should reopen");
        let state = store.state().expect("appearance state should be readable");
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(state.active_theme_id, "studio-dark");
        assert_eq!(state.accent_override.as_deref(), Some("#78d5b3"));
    }

    #[test]
    fn built_in_catalog_only_contains_studio_dark() {
        let (store, directory) = temp_store();

        let state = store.state().expect("appearance state should be readable");
        let built_ins: Vec<&ThemeCatalogItem> = state
            .themes
            .iter()
            .filter(|theme| theme.source == "builtIn")
            .collect();
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(built_ins.len(), 1);
        assert_eq!(built_ins[0].id, "studio-dark");
        assert_eq!(built_ins[0].name, "Studio Dark");
    }

    #[test]
    fn imports_theme_plugin_and_lists_enabled_theme() {
        let (mut store, directory) = temp_store();

        let state = store
            .import_theme_plugin_json(ocean_plugin_json())
            .expect("theme plugin manifest should import");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(state.plugins.iter().any(|plugin| {
            plugin.id == "dev.openplayer.theme.ocean" && plugin.enabled && plugin.theme_count == 1
        }));
        assert!(state.themes.iter().any(|theme| {
            theme.id == "dev.openplayer.theme.ocean.dark"
                && theme.source == "plugin"
                && theme.plugin_id.as_deref() == Some("dev.openplayer.theme.ocean")
                && theme.enabled
        }));
    }

    #[test]
    fn installs_manifest_file_into_managed_plugin_directory() {
        let (mut store, directory) = temp_store();
        let source_manifest = directory.join("source-subtitle-plugin.json");
        std::fs::write(&source_manifest, subtitle_plugin_json())
            .expect("source plugin manifest should be written");

        let state = store
            .import_plugin_manifest_path(&source_manifest)
            .expect("plugin manifest file should install");

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .expect("installed plugin should be listed");
        let install_path = plugin
            .install_path
            .as_ref()
            .expect("installed plugin should expose install path");
        let installed_directory = PathBuf::from(install_path);

        assert_eq!(plugin.package_kind, "manifestFile");
        assert!(plugin.installed_at_ms.unwrap_or_default() > 0);
        assert!(installed_directory.ends_with("dev.openplayer.subtitle.styler"));
        assert!(installed_directory.join("manifest.json").exists());
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn installs_plugin_directory_and_copies_package_assets() {
        let (mut store, directory) = temp_store();
        let source_directory = directory.join("subtitle-package");
        std::fs::create_dir_all(source_directory.join("assets"))
            .expect("source plugin package directory should be created");
        std::fs::write(
            source_directory.join("manifest.json"),
            subtitle_plugin_json(),
        )
        .expect("source plugin manifest should be written");
        std::fs::write(
            source_directory.join("assets").join("readme.txt"),
            "package asset",
        )
        .expect("source plugin asset should be written");

        let state = store
            .import_plugin_directory_path(&source_directory)
            .expect("plugin directory should install");

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .expect("installed plugin should be listed");
        let install_path = plugin
            .install_path
            .as_ref()
            .expect("installed plugin should expose install path");
        let installed_directory = PathBuf::from(install_path);

        assert_eq!(plugin.package_kind, "directory");
        assert!(installed_directory.join("manifest.json").exists());
        assert!(
            installed_directory
                .join("assets")
                .join("readme.txt")
                .exists()
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn installs_opplugin_package_and_extracts_assets() {
        let (mut store, directory) = temp_store();
        let package_path = directory.join("subtitle-styler.opplugin");
        write_opplugin_package(&package_path, subtitle_plugin_json());

        let state = store
            .import_plugin_package_path(&package_path)
            .expect("OpenPlayer plugin package should install");

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .expect("installed plugin should be listed");
        let install_path = plugin
            .install_path
            .as_ref()
            .expect("installed plugin should expose install path");
        let installed_directory = PathBuf::from(install_path);

        assert_eq!(plugin.package_kind, "opplugin");
        assert!(installed_directory.join("manifest.json").exists());
        assert!(
            installed_directory
                .join("assets")
                .join("readme.txt")
                .exists()
        );
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn imports_capability_plugin_without_theme_and_lists_settings() {
        let (mut store, directory) = temp_store();

        let state = store
            .import_theme_plugin_json(subtitle_plugin_json())
            .expect("capability plugin manifest should import");
        let _ = std::fs::remove_dir_all(&directory);

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .expect("subtitle plugin should be listed");
        assert!(plugin.enabled);
        assert_eq!(plugin.theme_count, 0);
        assert_eq!(plugin.capability_count, 1);
        assert_eq!(plugin.setting_count, 2);
        assert_eq!(plugin.permissions, vec!["mpv.subtitleStyle"]);
        assert_eq!(plugin.settings[0].value, serde_json::json!(42));
    }

    #[test]
    fn imports_extended_subtitle_typography_mpv_settings() {
        let (mut store, directory) = temp_store();

        let state = store
            .import_theme_plugin_json(extended_subtitle_plugin_json())
            .expect("extended subtitle typography plugin should import");
        let _ = std::fs::remove_dir_all(&directory);

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.typography")
            .expect("subtitle typography plugin should be listed");
        let mpv_properties: Vec<&str> = plugin
            .settings
            .iter()
            .filter_map(|setting| setting.mpv_property.as_deref())
            .collect();

        assert_eq!(
            mpv_properties,
            vec!["sub-spacing", "sub-outline-size", "sub-shadow-offset"]
        );
    }

    #[test]
    fn imports_documented_subtitle_typography_example_plugin() {
        let (mut store, directory) = temp_store();
        let manifest = include_str!("../fixtures/plugins/subtitle-typography/manifest.json");

        let state = store
            .import_theme_plugin_json(manifest)
            .expect("documented subtitle typography example should import");
        let _ = std::fs::remove_dir_all(&directory);

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.typography")
            .expect("example subtitle typography plugin should be listed");

        assert_eq!(plugin.setting_count, 8);
        assert_eq!(plugin.action_count, 0);
        assert!(
            !plugin
                .settings
                .iter()
                .any(|setting| setting.mpv_property.as_deref() == Some("sub-line-spacing"))
        );
        let letter_spacing = plugin
            .settings
            .iter()
            .find(|setting| setting.id == "letter-spacing")
            .expect("letter spacing setting should exist");
        assert_eq!(letter_spacing.max, Some(10.0));
        let font_size = plugin
            .settings
            .iter()
            .find(|setting| setting.id == "font-size")
            .expect("font size setting should exist");
        assert_eq!(
            font_size.label_i18n.get("zh-CN").map(String::as_str),
            Some("字号")
        );
    }

    #[test]
    fn rejects_removed_subtitle_line_spacing_mpv_property() {
        let (mut store, directory) = temp_store();

        let error = store
            .import_theme_plugin_json(
                r##"{
                  "id": "dev.openplayer.subtitle.line-spacing",
                  "name": "Removed Subtitle Line Spacing",
                  "version": "1.0.0",
                  "entry": "manifest",
                  "runtime": { "kind": "manifest" },
                  "contributes": {
                    "settings": [
                      {
                        "id": "line-spacing",
                        "label": "Line Spacing",
                        "kind": "number",
                        "placement": "subtitleSettings",
                        "defaultValue": 0,
                        "min": -10,
                        "max": 10,
                        "step": 1,
                        "mpvProperty": "sub-line-spacing"
                      }
                    ]
                  }
                }"##,
            )
            .expect_err("removed subtitle line spacing property should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("unsupported plugin mpv property: sub-line-spacing"));
    }

    #[test]
    fn imports_plugin_actions_for_ui_slots() {
        let (mut store, directory) = temp_store();

        let state = store
            .import_theme_plugin_json(action_plugin_json())
            .expect("action plugin manifest should import");
        let _ = std::fs::remove_dir_all(&directory);

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.quick.actions")
            .expect("action plugin should be listed");
        assert_eq!(plugin.action_count, 2);
        assert_eq!(plugin.actions[0].id, "toggle-tracks");
        assert_eq!(plugin.actions[0].placement, "controls.right");
        assert_eq!(plugin.actions[0].command, "player.toggleTracks");
        assert!(plugin.actions[0].requires_media);
        assert_eq!(plugin.actions[1].placement, "contextMenu");
    }

    #[test]
    fn imports_capability_actions_with_valid_permissions_and_args() {
        let (mut store, directory) = temp_store();

        let state = store
            .import_theme_plugin_json(capability_action_plugin_json())
            .expect("capability actions should import");
        let _ = std::fs::remove_dir_all(&directory);

        let plugin = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.capability.actions")
            .expect("capability action plugin should be listed");
        assert_eq!(plugin.action_count, 2);
        assert_eq!(plugin.actions[0].command, "player.captureScreenshot");
        assert_eq!(
            plugin.actions[0].args,
            serde_json::json!({ "openFolder": true })
        );
        assert_eq!(plugin.actions[1].command, "player.openStream");
        assert_eq!(
            plugin.actions[1].args,
            serde_json::json!({ "url": "https://example.com/live.m3u8", "name": "Live Stream" })
        );
    }

    #[test]
    fn rejects_capability_actions_without_required_permissions() {
        let (mut store, directory) = temp_store();
        let invalid = capability_action_plugin_json()
            .replace("\"permissions\": [\"mpv.capture\"]", "\"permissions\": []");

        let error = store
            .import_theme_plugin_json(&invalid)
            .expect_err("capture action without permission should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("requires permission mpv.capture"));
    }

    #[test]
    fn rejects_plugin_stream_actions_with_unsafe_urls() {
        let (mut store, directory) = temp_store();
        let invalid = capability_action_plugin_json()
            .replace("https://example.com/live.m3u8", "file://C:/secret.mp4");

        let error = store
            .import_theme_plugin_json(&invalid)
            .expect_err("unsafe stream urls should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("unsupported plugin stream protocol"));
    }

    #[test]
    fn uninstalling_plugin_removes_state_settings_and_installed_files() {
        let (mut store, directory) = temp_store();
        let source_manifest = directory.join("subtitle-plugin.json");
        std::fs::write(&source_manifest, subtitle_plugin_json())
            .expect("source plugin manifest should be written");
        let installed = store
            .import_plugin_manifest_path(&source_manifest)
            .expect("plugin manifest file should install");
        let install_path = installed
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .and_then(|plugin| plugin.install_path.clone())
            .expect("installed plugin should expose install path");
        store
            .set_plugin_setting(
                "dev.openplayer.subtitle.styler",
                "font-size",
                serde_json::json!(56),
            )
            .expect("valid plugin setting should persist");

        let state = store
            .uninstall_plugin("dev.openplayer.subtitle.styler")
            .expect("plugin should uninstall");
        assert!(
            !state
                .plugins
                .iter()
                .any(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
        );
        assert!(!PathBuf::from(&install_path).exists());

        let reinstalled = store
            .import_plugin_manifest_path(&source_manifest)
            .expect("plugin manifest file should reinstall");
        let font_size = reinstalled
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .and_then(|plugin| {
                plugin
                    .settings
                    .iter()
                    .find(|setting| setting.id == "font-size")
            })
            .map(|setting| setting.value.clone());
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(font_size, Some(serde_json::json!(42)));
    }

    #[test]
    fn rejects_plugin_actions_with_unsupported_commands() {
        let (mut store, directory) = temp_store();
        let invalid = action_plugin_json().replace("player.toggleTracks", "system.exec");

        let error = store
            .import_theme_plugin_json(&invalid)
            .expect_err("unsupported action commands should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("unsupported plugin action command"));
    }

    #[test]
    fn persists_valid_plugin_setting_values() {
        let (mut store, directory) = temp_store();
        store
            .import_theme_plugin_json(subtitle_plugin_json())
            .expect("capability plugin manifest should import");

        let state = store
            .set_plugin_setting(
                "dev.openplayer.subtitle.styler",
                "font-size",
                serde_json::json!(56),
            )
            .expect("valid plugin setting should persist");
        drop(store);

        let reopened = AppearanceStore::open(directory.join("settings.redb"))
            .expect("appearance store should reopen");
        let reopened_state = reopened
            .state()
            .expect("appearance state should be readable");
        let _ = std::fs::remove_dir_all(&directory);

        let state_value = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .and_then(|plugin| {
                plugin
                    .settings
                    .iter()
                    .find(|setting| setting.id == "font-size")
            })
            .map(|setting| setting.value.clone());
        let reopened_value = reopened_state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .and_then(|plugin| {
                plugin
                    .settings
                    .iter()
                    .find(|setting| setting.id == "font-size")
            })
            .map(|setting| setting.value.clone());

        assert_eq!(state_value, Some(serde_json::json!(56)));
        assert_eq!(reopened_value, Some(serde_json::json!(56)));
    }

    #[test]
    fn rejects_plugin_setting_values_outside_schema() {
        let (mut store, directory) = temp_store();
        store
            .import_theme_plugin_json(subtitle_plugin_json())
            .expect("capability plugin manifest should import");

        let error = store
            .set_plugin_setting(
                "dev.openplayer.subtitle.styler",
                "font-size",
                serde_json::json!(120),
            )
            .expect_err("out-of-range plugin setting should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("font-size"));
    }

    #[test]
    fn falls_back_to_default_when_stored_plugin_setting_no_longer_matches_schema() {
        let (mut store, directory) = temp_store();
        store
            .import_theme_plugin_json(subtitle_plugin_json())
            .expect("capability plugin manifest should import");
        store
            .set_plugin_setting(
                "dev.openplayer.subtitle.styler",
                "font-size",
                serde_json::json!(56),
            )
            .expect("valid plugin setting should persist");
        let updated_manifest = subtitle_plugin_json().replace("\"max\": 96", "\"max\": 48");
        let state = store
            .import_theme_plugin_json(&updated_manifest)
            .expect("plugin update should import with stricter schema");
        let _ = std::fs::remove_dir_all(&directory);

        let value = state
            .plugins
            .iter()
            .find(|plugin| plugin.id == "dev.openplayer.subtitle.styler")
            .and_then(|plugin| {
                plugin
                    .settings
                    .iter()
                    .find(|setting| setting.id == "font-size")
            })
            .map(|setting| setting.value.clone());

        assert_eq!(value, Some(serde_json::json!(42)));
    }

    #[test]
    fn imports_webview_runtime_source_from_installed_plugin_package() {
        let (mut store, directory) = temp_store();
        let source_directory = directory.join("worker-runtime");
        std::fs::create_dir_all(source_directory.join("dist"))
            .expect("runtime package directory should be created");
        std::fs::write(
            source_directory.join("manifest.json"),
            webview_runtime_plugin_json(),
        )
        .expect("runtime plugin manifest should be written");
        std::fs::write(
            source_directory.join("dist").join("plugin.js"),
            "openplayer.request('player.captureScreenshot', { openFolder: false });",
        )
        .expect("runtime plugin script should be written");

        store
            .import_plugin_directory_path(&source_directory)
            .expect("webview runtime plugin should import");
        let sources = store
            .plugin_runtime_sources()
            .expect("runtime sources should be readable");
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].plugin_id, "dev.openplayer.runtime.worker");
        assert_eq!(sources[0].entry, "dist/plugin.js");
        assert!(sources[0].script.contains("player.captureScreenshot"));
        assert_eq!(sources[0].permissions, vec!["mpv.capture"]);
    }

    #[test]
    fn rejects_webview_runtime_without_entry() {
        let (mut store, directory) = temp_store();
        let invalid = webview_runtime_plugin_json().replace(
            "\"entry\": \"dist/plugin.js\",\n            \"sandbox\": \"openplayer-worker\"",
            "\"sandbox\": \"openplayer-worker\"",
        );

        let error = store
            .import_theme_plugin_json(&invalid)
            .expect_err("webview runtime without entry should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("webviewJs"));
        assert!(error.contains("entry"));
    }

    #[test]
    fn rejects_wasm_plugin_runtimes_until_wasm_sandbox_exists() {
        let (mut store, directory) = temp_store();

        let error = store
            .import_theme_plugin_json(wasm_runtime_plugin_json())
            .expect_err("wasm plugin runtimes should be rejected for now");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("wasm"));
        assert!(error.contains("not supported yet"));
    }

    #[test]
    fn disabling_active_plugin_theme_falls_back_to_studio_dark() {
        let (mut store, directory) = temp_store();
        store
            .import_theme_plugin_json(ocean_plugin_json())
            .expect("theme plugin manifest should import");
        store
            .set_theme("dev.openplayer.theme.ocean.dark")
            .expect("plugin theme should be selectable");

        let state = store
            .set_plugin_enabled("dev.openplayer.theme.ocean", false)
            .expect("theme plugin should be disabled");
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(state.active_theme_id, "studio-dark");
        assert!(
            state
                .themes
                .iter()
                .any(|theme| theme.id == "dev.openplayer.theme.ocean.dark" && !theme.enabled)
        );
    }

    #[test]
    fn rejects_invalid_theme_plugin_color() {
        let (mut store, directory) = temp_store();
        let invalid = ocean_plugin_json().replace("\"#62c7b7\"", "\"blue\"");

        let error = store
            .import_theme_plugin_json(&invalid)
            .expect_err("invalid color token should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("accent"));
    }

    #[test]
    fn player_preferences_default_false_and_persist() {
        let (mut store, directory) = temp_store();

        assert_eq!(
            store.preferences().expect("preferences should be readable"),
            PlayerPreferences {
                incognito_mode: false,
                quiet_keyboard_controls: false,
                language_mode: "system".to_string(),
            }
        );

        store
            .set_bool_preference(INCOGNITO_MODE_KEY, true)
            .expect("incognito mode should be persisted");
        store
            .set_bool_preference(QUIET_KEYBOARD_CONTROLS_KEY, true)
            .expect("quiet keyboard controls should be persisted");
        let preferences = store
            .set_language_mode("en-US")
            .expect("language mode should be persisted");
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(
            preferences,
            PlayerPreferences {
                incognito_mode: true,
                quiet_keyboard_controls: true,
                language_mode: "en-US".to_string(),
            }
        );
    }

    #[test]
    fn rejects_invalid_language_mode_preference() {
        let (mut store, directory) = temp_store();

        let error = store
            .set_language_mode("fr-FR")
            .expect_err("unsupported language modes should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(error, "invalid language mode");
    }
}
