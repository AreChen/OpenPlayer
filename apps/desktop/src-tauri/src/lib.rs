#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;

#[cfg(all(not(windows), feature = "mpv-embed"))]
mod mpv_embed;

#[cfg(not(windows))]
mod app_info;
#[cfg(not(windows))]
mod appearance_store;
#[cfg(not(windows))]
mod bootstrap;
#[cfg(not(windows))]
mod external_open;
mod media_paths;
#[cfg(not(windows))]
mod native_shortcuts;
#[cfg(not(windows))]
mod platform_support;
mod playback_store;
#[cfg(not(windows))]
mod plugin_artifacts;
#[cfg(not(windows))]
mod plugin_native;
#[cfg(not(windows))]
mod plugin_network;
mod shell_preview;
mod store_schema;
#[cfg(not(windows))]
mod system_fonts;
#[cfg(not(windows))]
mod window;
#[cfg(all(windows, feature = "windows-gpui"))]
mod windows_gpui;

#[cfg(not(windows))]
pub use bootstrap::run;
#[cfg(feature = "mpv-smoke")]
pub use mpv_smoke::{MpvSmokeReport, create_headless_probe};
#[cfg(all(not(windows), feature = "window-smoke"))]
pub use window::smoke::run as run_window_smoke;
#[cfg(all(windows, feature = "windows-gpui"))]
pub use windows_gpui::run;

#[cfg(all(windows, not(feature = "windows-gpui")))]
compile_error!("The Windows player requires the windows-gpui feature.");
