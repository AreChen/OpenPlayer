use std::{collections::HashSet, fs, path::PathBuf, sync::Mutex, thread, time::Duration};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const SETTINGS_KV: TableDefinition<&str, &str> = TableDefinition::new("settings_kv");
const THEME_MANIFESTS: TableDefinition<&str, &str> = TableDefinition::new("theme_manifests");
const PLUGIN_MANIFESTS: TableDefinition<&str, &str> = TableDefinition::new("plugin_manifests");
const PLUGIN_ENABLEMENT: TableDefinition<&str, &str> = TableDefinition::new("plugin_enablement");
const ACTIVE_THEME_KEY: &str = "activeThemeId";
const ACCENT_OVERRIDE_KEY: &str = "accentOverride";
const INCOGNITO_MODE_KEY: &str = "incognitoMode";
const QUIET_KEYBOARD_CONTROLS_KEY: &str = "quietKeyboardControls";
const LANGUAGE_MODE_KEY: &str = "languageMode";
const DEFAULT_THEME_ID: &str = "studio-dark";

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
struct ThemePluginManifest {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    entry: ThemePluginEntry,
    contributes: ThemePluginContributions,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ThemePluginContributions {
    themes: Vec<ThemeManifest>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
enum ThemePluginEntry {
    #[serde(rename = "manifest")]
    Manifest,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePluginSummary {
    id: String,
    name: String,
    version: String,
    description: Option<String>,
    enabled: bool,
    theme_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

        let database = create_database_with_retry(&path, "appearance settings")?;
        let store = Self { database };
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

        let mut plugins = Vec::new();
        for item in plugin_manifests
            .iter()
            .map_err(|error| format!("failed to scan plugin manifests: {error}"))?
        {
            let (_, value) =
                item.map_err(|error| format!("failed to read plugin manifest: {error}"))?;
            let manifest = decode_plugin_manifest(value.value())?;
            let enabled = plugin_enabled_from_table(&plugin_enablement, &manifest.id)?;
            plugins.push(ThemePluginSummary {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
                description: manifest.description,
                enabled,
                theme_count: manifest.contributes.themes.len(),
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

    fn import_theme_plugin_json(&mut self, json: &str) -> Result<AppearanceState, String> {
        let manifest = parse_theme_plugin_manifest_json(json)?;
        let encoded_plugin = serde_json::to_string(&manifest)
            .map_err(|error| format!("failed to encode theme plugin manifest: {error}"))?;

        let transaction = self
            .database
            .begin_write()
            .map_err(|error| format!("failed to import theme plugin: {error}"))?;
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
            .map_err(|error| format!("failed to commit theme plugin import: {error}"))?;
        self.state()
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

fn parse_theme_plugin_manifest_json(json: &str) -> Result<ThemePluginManifest, String> {
    let manifest: ThemePluginManifest = serde_json::from_str(json)
        .map_err(|error| format!("invalid theme plugin manifest JSON: {error}"))?;
    validate_plugin_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_plugin_manifest(manifest: &ThemePluginManifest) -> Result<(), String> {
    validate_non_empty("plugin id", &manifest.id)?;
    validate_non_empty("plugin name", &manifest.name)?;
    validate_non_empty("plugin version", &manifest.version)?;
    validate_dotted_identifier("plugin id", &manifest.id, true)?;
    validate_simple_semver("plugin version", &manifest.version)?;
    if let Some(description) = manifest.description.as_deref() {
        validate_non_empty("plugin description", description)?;
    }
    if manifest.contributes.themes.is_empty() {
        return Err("theme plugin must contribute at least one theme".to_string());
    }

    let mut ids = HashSet::new();
    for theme in &manifest.contributes.themes {
        validate_theme_manifest(theme)?;
        if !ids.insert(theme.id.as_str()) {
            return Err(format!("duplicate theme id: {}", theme.id));
        }
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

fn decode_plugin_manifest(value: &str) -> Result<ThemePluginManifest, String> {
    serde_json::from_str(value)
        .map_err(|error| format!("failed to decode plugin manifest: {error}"))
}

fn decode_stored_theme_manifest(value: &str) -> Result<StoredThemeManifest, String> {
    serde_json::from_str(value).map_err(|error| format!("failed to decode theme manifest: {error}"))
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
pub fn appearance_import_theme_plugin(
    state: State<'_, AppearanceStoreState>,
    path: String,
) -> Result<AppearanceState, String> {
    let json = std::fs::read_to_string(path.trim())
        .map_err(|error| format!("failed to read theme plugin manifest: {error}"))?;
    state.with_store(|store| store.import_theme_plugin_json(&json))
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
pub fn appearance_reset(state: State<'_, AppearanceStoreState>) -> Result<AppearanceState, String> {
    state.with_store(|store| store.reset())
}

#[cfg(test)]
mod tests {
    use super::*;
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
