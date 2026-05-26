use super::*;

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
