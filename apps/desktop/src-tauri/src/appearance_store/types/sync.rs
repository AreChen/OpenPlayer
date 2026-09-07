use super::{AppearanceState, PlayerPreferences};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSyncState {
    pub(in crate::appearance_store) appearance: AppearanceState,
    pub(in crate::appearance_store) preferences: PlayerPreferences,
}
