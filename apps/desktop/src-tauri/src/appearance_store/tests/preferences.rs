use super::*;

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
