use std::{collections::HashMap, path::PathBuf};

use serde_json::Value;
use tauri::State;

use super::{
    INCOGNITO_MODE_KEY, QUIET_KEYBOARD_CONTROLS_KEY, store::AppearanceStoreState, types::*,
};
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
pub fn appearance_plugin_view_html(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
    view_id: String,
) -> Result<PluginViewHtml, String> {
    state.with_store(|store| store.plugin_view_html(&plugin_id, &view_id))
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
pub fn appearance_plugin_kv_get(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
    key: String,
) -> Result<Option<Value>, String> {
    state.with_store(|store| store.plugin_runtime_storage_value(&plugin_id, &key))
}

#[tauri::command]
pub fn appearance_plugin_kv_list(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
) -> Result<HashMap<String, Value>, String> {
    state.with_store(|store| store.plugin_runtime_storage_values(&plugin_id))
}

#[tauri::command]
pub fn appearance_plugin_kv_set(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
    key: String,
    value: Value,
) -> Result<(), String> {
    state.with_store(|store| store.set_plugin_runtime_storage_value(&plugin_id, &key, value))
}

#[tauri::command]
pub fn appearance_plugin_kv_remove(
    state: State<'_, AppearanceStoreState>,
    plugin_id: String,
    key: String,
) -> Result<bool, String> {
    state.with_store(|store| store.remove_plugin_runtime_storage_value(&plugin_id, &key))
}

#[tauri::command]
pub fn appearance_reset(state: State<'_, AppearanceStoreState>) -> Result<AppearanceState, String> {
    state.with_store(|store| store.reset())
}
