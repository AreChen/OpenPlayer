use super::{
    ACCENT_OVERRIDE_KEY, ACTIVE_THEME_KEY, DEFAULT_THEME_ID, SETTINGS_KV,
    manifest::validate_color_token, store::AppearanceStore, types::AppearanceState,
};

impl AppearanceStore {
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
