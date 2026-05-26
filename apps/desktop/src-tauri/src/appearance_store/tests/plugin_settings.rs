use super::*;

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
    let manifest = include_str!("../../../fixtures/plugins/subtitle-typography/manifest.json");

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
