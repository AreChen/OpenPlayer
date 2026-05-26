use redb::TableDefinition;

mod commands;
mod database;
mod manifest;
mod package;
mod records;
mod store;
#[cfg(test)]
mod tests;
mod themes;
mod types;

pub use commands::{
    appearance_import_plugin_directory, appearance_import_plugin_manifest,
    appearance_import_plugin_package, appearance_import_theme_plugin, appearance_plugin_kv_get,
    appearance_plugin_kv_list, appearance_plugin_kv_remove, appearance_plugin_kv_set,
    appearance_plugin_runtime_sources, appearance_plugin_view_html, appearance_reset,
    appearance_set_accent_override, appearance_set_plugin_enabled, appearance_set_plugin_setting,
    appearance_set_theme, appearance_state, appearance_uninstall_plugin,
    preferences_set_incognito_mode, preferences_set_language_mode,
    preferences_set_quiet_keyboard_controls, preferences_state,
};
pub use store::AppearanceStoreState;

const SETTINGS_KV: TableDefinition<&str, &str> = TableDefinition::new("settings_kv");
const THEME_MANIFESTS: TableDefinition<&str, &str> = TableDefinition::new("theme_manifests");
const PLUGIN_MANIFESTS: TableDefinition<&str, &str> = TableDefinition::new("plugin_manifests");
const PLUGIN_ENABLEMENT: TableDefinition<&str, &str> = TableDefinition::new("plugin_enablement");
const PLUGIN_SETTINGS: TableDefinition<&str, &str> = TableDefinition::new("plugin_settings");
const PLUGIN_RUNTIME_STORAGE: TableDefinition<&str, &str> =
    TableDefinition::new("plugin_runtime_storage");
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
const MAX_PLUGIN_VIEW_HTML_BYTES: u64 = 2 * 1024 * 1024;
const MAX_PLUGIN_RUNTIME_STORAGE_KEY_BYTES: usize = 128;
const MAX_PLUGIN_RUNTIME_STORAGE_VALUE_BYTES: usize = 64 * 1024;
const SUPPORTED_PLUGIN_API_VERSION: &str = "1";
