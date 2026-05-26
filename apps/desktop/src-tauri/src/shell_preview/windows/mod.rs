#[cfg(windows)]
mod registration;
#[cfg(windows)]
mod registry;
#[cfg(windows)]
mod settings;

#[cfg(not(windows))]
use super::formats::{ShellPreviewFormat, ShellPreviewRegistrationSummary};

#[cfg(windows)]
pub(super) use registration::register_shell_previews;
#[cfg(windows)]
pub(super) use settings::open_default_apps_settings;

#[cfg(not(windows))]
pub(super) fn register_shell_previews(
    _app: &tauri::AppHandle,
    _formats: &[ShellPreviewFormat],
) -> Result<ShellPreviewRegistrationSummary, String> {
    Err("Explorer preview registration is only available on Windows".to_string())
}

#[cfg(not(windows))]
pub(super) fn open_default_apps_settings() -> Result<(), String> {
    Err("Windows default apps settings are only available on Windows".to_string())
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
