use openplayer_shared::AppInfo;
use tauri::Window;

pub fn app_health() -> AppInfo {
    openplayer_core::app_info()
}

#[tauri::command(rename = "app_health")]
fn app_health_command() -> AppInfo {
    app_health()
}

#[tauri::command]
fn window_minimize(window: Window) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

#[tauri::command]
fn window_toggle_maximize(window: Window) -> Result<(), String> {
    if window.is_maximized().map_err(|error| error.to_string())? {
        window.unmaximize().map_err(|error| error.to_string())
    } else {
        window.maximize().map_err(|error| error.to_string())
    }
}

#[tauri::command]
fn window_close(window: Window) -> Result<(), String> {
    window.close().map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_health_command,
            window_minimize,
            window_toggle_maximize,
            window_close
        ])
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
