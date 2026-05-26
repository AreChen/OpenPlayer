use redb::ReadableDatabase;

use super::{
    INCOGNITO_MODE_KEY, LANGUAGE_MODE_KEY, QUIET_KEYBOARD_CONTROLS_KEY, SETTINGS_KV,
    records::{read_bool_setting, read_language_mode_setting, validate_language_mode},
    store::AppearanceStore,
    types::PlayerPreferences,
};

impl AppearanceStore {
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
}
