mod errors;
mod factory;
mod locale;
mod logging;
#[cfg(target_os = "macos")]
mod macos_render;
mod platform;

pub(super) use errors::mpv_error_message;
pub(super) use factory::create_embed_player;
#[cfg(windows)]
pub(super) use factory::create_embed_player_without_logs;
pub(super) use locale::prepare_libmpv_numeric_locale;
#[cfg(test)]
pub(super) use logging::is_mpv_video_diagnostic_log;
pub(super) use logging::log_mpv_video_diagnostic;
#[cfg(target_os = "macos")]
pub(super) use macos_render::create_macos_render_context;
#[cfg(all(test, target_os = "macos"))]
pub(super) use platform::platform_video_output_config;
#[cfg(test)]
pub(super) use platform::{is_virtual_linux_drm_driver, resolve_linux_video_output_config};
