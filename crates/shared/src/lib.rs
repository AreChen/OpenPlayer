use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub stage: AppStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppStage {
    Skeleton,
}

impl AppInfo {
    pub fn skeleton(version: impl Into<String>) -> Self {
        Self {
            name: "OpenPlayer".to_string(),
            version: version.into(),
            stage: AppStage::Skeleton,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_app_info_for_tauri_ipc() {
        let info = AppInfo::skeleton("0.1.0");
        let json = serde_json::to_value(info).expect("app info serializes");

        assert_eq!(json["name"], "OpenPlayer");
        assert_eq!(json["version"], "0.1.0");
        assert_eq!(json["stage"], "skeleton");
    }
}
