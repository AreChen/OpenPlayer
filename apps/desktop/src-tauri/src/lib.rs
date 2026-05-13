use openplayer_shared::AppInfo;

pub fn app_health() -> AppInfo {
    openplayer_core::app_info()
}

#[tauri::command(rename = "app_health")]
fn app_health_command() -> AppInfo {
    app_health()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_health_command])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_shared::AppStage;

    #[test]
    fn app_health_reports_core_info() {
        let info = app_health();

        assert_eq!(info.name, "OpenPlayer");
        assert_eq!(info.stage, AppStage::Skeleton);
    }
}
