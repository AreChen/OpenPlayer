use tauri::Manager;

use crate::appearance_store::AppearanceStoreState;
use crate::media_paths::StartupMediaState;
use crate::mpv_embed::{MpvEmbedState, MpvWallState};
use crate::platform_support::prepare_platform_runtime;
use crate::playback_store::PlaybackStoreState;
use crate::{native_shortcuts, window};

pub fn run() {
    prepare_platform_runtime();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(window::WindowState::default())
        .manage(MpvEmbedState::default())
        .manage(MpvWallState::default())
        .manage(StartupMediaState::from_args(std::env::args_os()))
        .setup(|app| {
            app.manage(AppearanceStoreState::open(app.handle()));
            app.manage(PlaybackStoreState::open(app.handle()));
            native_shortcuts::install_native_shortcut_hook(app.handle().clone());
            window::setup_overlay_window(app)?;
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
            crate::window::window_close,
            crate::window::window_focus_overlay,
            crate::window::window_start_drag,
            crate::window::window_start_resize,
            crate::window::window_set_resize_cursor,
            crate::window::window_apply_resize_delta,
            crate::window::window_reveal_path,
            crate::window::window_open_directory,
            crate::window::mpv_overlay_open_path,
            crate::mpv_embed::mpv_embed_play,
            crate::mpv_embed::mpv_embed_pause,
            crate::mpv_embed::mpv_embed_seek,
            crate::mpv_embed::mpv_embed_frame_step,
            crate::mpv_embed::mpv_embed_frame_back_step,
            crate::mpv_embed::mpv_embed_set_hwdec,
            crate::mpv_embed::mpv_embed_set_loop_file,
            crate::mpv_embed::mpv_embed_set_speed,
            crate::mpv_embed::mpv_embed_set_video_fill,
            crate::mpv_embed::mpv_embed_set_subtitle_delay,
            crate::mpv_embed::mpv_embed_set_plugin_property,
            crate::mpv_embed::mpv_embed_plugin_get_property,
            crate::mpv_embed::mpv_embed_plugin_set_property,
            crate::mpv_embed::mpv_embed_plugin_set_ab_loop,
            crate::mpv_embed::mpv_embed_plugin_clear_ab_loop,
            crate::mpv_embed::mpv_embed_plugin_command,
            crate::mpv_embed::mpv_embed_plugin_add_video_filter,
            crate::mpv_embed::mpv_embed_plugin_remove_video_filter,
            crate::mpv_embed::mpv_embed_plugin_add_audio_filter,
            crate::mpv_embed::mpv_embed_plugin_remove_audio_filter,
            crate::mpv_embed::mpv_embed_extract_audio_clip,
            crate::mpv_embed::mpv_embed_capture_screenshot,
            crate::mpv_embed::mpv_embed_recording_state,
            crate::mpv_embed::mpv_embed_start_recording,
            crate::mpv_embed::mpv_embed_stop_recording,
            crate::mpv_embed::mpv_embed_select_track,
            crate::mpv_embed::mpv_embed_add_subtitle,
            crate::mpv_embed::mpv_embed_load_generated_subtitle,
            crate::mpv_embed::mpv_embed_list_generated_subtitles,
            crate::mpv_embed::mpv_embed_remove_generated_subtitle,
            crate::mpv_embed::mpv_embed_replace_generated_subtitle,
            crate::mpv_embed::mpv_embed_set_volume,
            crate::mpv_embed::mpv_embed_snapshot,
            crate::mpv_embed::mpv_embed_stop,
            crate::mpv_embed::mpv_wall_open,
            crate::mpv_embed::mpv_wall_layout,
            crate::mpv_embed::mpv_wall_snapshot,
            crate::mpv_embed::mpv_wall_close,
            crate::mpv_embed::mpv_wall_set_visible,
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
