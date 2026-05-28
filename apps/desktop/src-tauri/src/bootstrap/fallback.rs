use tauri::Manager;

use crate::appearance_store::AppearanceStoreState;
use crate::media_paths::StartupMediaState;
use crate::platform_support::prepare_platform_runtime;
use crate::playback_store::PlaybackStoreState;
use crate::window;

pub fn run() {
    prepare_platform_runtime();
    tauri::Builder::default()
        .manage(window::WindowState::default())
        .manage(StartupMediaState::from_args(std::env::args_os()))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(AppearanceStoreState::open(app.handle()));
            app.manage(PlaybackStoreState::open(app.handle()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::native_shortcuts::window_update_shortcuts,
            crate::native_shortcuts::window_set_shortcuts_enabled,
            crate::app_info::app_version,
            crate::system_fonts::system_font_families,
            crate::external_open::app_open_url,
            crate::window::window_minimize,
            crate::window::window_toggle_maximize,
            crate::window::window_toggle_fullscreen,
            crate::window::window_always_on_top_state,
            crate::window::window_toggle_always_on_top,
            crate::window::window_focus_overlay,
            crate::window::window_start_resize,
            crate::window::window_set_resize_cursor,
            crate::window::window_apply_resize_delta,
            crate::window::window_close,
            crate::window::window_reveal_path,
            crate::window::window_open_directory,
            crate::media_paths::commands::media_files_from_paths,
            crate::media_paths::commands::media_files_in_directory,
            crate::media_paths::commands::startup_media_paths,
            crate::platform_support::platform_support,
            crate::appearance_store::commands::appearance_state,
            crate::appearance_store::commands::appearance_set_theme,
            crate::appearance_store::commands::appearance_set_accent_override,
            crate::appearance_store::commands::appearance_import_plugin_manifest,
            crate::appearance_store::commands::appearance_import_plugin_package,
            crate::appearance_store::commands::appearance_import_plugin_directory,
            crate::appearance_store::commands::appearance_import_theme_plugin,
            crate::appearance_store::commands::appearance_plugin_runtime_sources,
            crate::appearance_store::commands::appearance_plugin_view_html,
            crate::appearance_store::commands::appearance_set_plugin_enabled,
            crate::appearance_store::commands::appearance_set_plugin_setting,
            crate::appearance_store::commands::appearance_plugin_kv_get,
            crate::appearance_store::commands::appearance_plugin_kv_list,
            crate::appearance_store::commands::appearance_plugin_kv_info,
            crate::appearance_store::commands::appearance_plugin_kv_mark_migrated,
            crate::appearance_store::commands::appearance_plugin_kv_set,
            crate::appearance_store::commands::appearance_plugin_kv_remove,
            crate::appearance_store::commands::appearance_uninstall_plugin,
            crate::plugin_network::plugin_network_request,
            crate::appearance_store::commands::appearance_reset,
            crate::appearance_store::commands::preferences_state,
            crate::appearance_store::commands::preferences_set_incognito_mode,
            crate::appearance_store::commands::preferences_set_quiet_keyboard_controls,
            crate::appearance_store::commands::preferences_set_language_mode,
            crate::shell_preview::shell_preview_formats,
            crate::shell_preview::shell_preview_open_default_apps_settings,
            crate::shell_preview::shell_preview_register_formats,
            crate::playback_store::commands::history_list,
            crate::playback_store::commands::history_remember,
            crate::playback_store::commands::history_resume_position,
            crate::playback_store::commands::history_clear,
            crate::playback_store::commands::network_stream_history_list,
            crate::playback_store::commands::network_stream_history_remember,
            crate::playback_store::commands::network_stream_history_clear,
            crate::playback_store::commands::playback_settings_state,
            crate::playback_store::commands::playback_settings_update,
            crate::playback_store::commands::playback_media_settings,
            crate::playback_store::commands::playback_media_settings_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
