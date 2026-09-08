#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;

#[cfg(feature = "mpv-embed")]
mod mpv_embed;

mod app_info;
mod appearance_store;
mod bootstrap;
mod external_open;
mod media_paths;
mod native_shortcuts;
mod platform_support;
mod playback_store;
mod plugin_artifacts;
mod plugin_native;
mod plugin_network;
mod shell_preview;
mod store_schema;
mod system_fonts;
mod window;

pub use bootstrap::run;
#[cfg(feature = "mpv-smoke")]
pub use mpv_smoke::{MpvSmokeReport, create_headless_probe};
#[cfg(feature = "window-smoke")]
pub use window::smoke::run as run_window_smoke;
