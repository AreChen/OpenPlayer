mod playback;
mod storage;

use openplayer_shared::AppInfo;
use playback::{
    DesktopPlaybackState, playback_open_preview_source, playback_pause, playback_play,
    playback_seek, playback_set_volume, playback_snapshot, playback_stop,
};
use storage::{
    DesktopStorageState, storage_progress_clear, storage_progress_get, storage_progress_save,
    storage_recent_media_list, storage_recent_media_record, storage_setting_get,
    storage_setting_set,
};
use tauri::{Manager, Window};

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let storage = match app.path().app_data_dir() {
                Ok(app_data_dir) => match std::fs::create_dir_all(&app_data_dir) {
                    Ok(()) => DesktopStorageState::open(app_data_dir.join("openplayer.sqlite3")),
                    Err(error) => DesktopStorageState::unavailable(error.to_string()),
                },
                Err(error) => DesktopStorageState::unavailable(error.to_string()),
            };
            app.manage(storage);
            Ok(())
        })
        .manage(DesktopPlaybackState::default())
        .invoke_handler(tauri::generate_handler![
            app_health_command,
            window_minimize,
            window_toggle_maximize,
            window_close,
            playback_snapshot,
            playback_open_preview_source,
            playback_play,
            playback_pause,
            playback_stop,
            playback_seek,
            playback_set_volume,
            storage_recent_media_list,
            storage_recent_media_record,
            storage_progress_get,
            storage_progress_save,
            storage_progress_clear,
            storage_setting_get,
            storage_setting_set
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
