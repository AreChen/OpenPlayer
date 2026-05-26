use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPreferences {
    pub(in crate::appearance_store) incognito_mode: bool,
    pub(in crate::appearance_store) quiet_keyboard_controls: bool,
    pub(in crate::appearance_store) language_mode: String,
}
