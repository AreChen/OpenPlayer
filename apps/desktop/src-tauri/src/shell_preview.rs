mod formats;
mod windows;

use formats::{PREVIEW_FORMATS, ShellPreviewFormat, filter_preview_formats};
pub use formats::{ShellPreviewFormatInfo, ShellPreviewRegistrationSummary};
use windows::{open_default_apps_settings, register_shell_previews};

#[cfg_attr(not(windows), tauri::command)]
pub fn shell_preview_formats() -> Vec<ShellPreviewFormatInfo> {
    PREVIEW_FORMATS
        .iter()
        .copied()
        .map(ShellPreviewFormat::info)
        .collect()
}

#[cfg_attr(not(windows), tauri::command)]
pub fn shell_preview_open_default_apps_settings() -> Result<(), String> {
    open_default_apps_settings()
}

#[cfg_attr(not(windows), tauri::command)]
pub fn shell_preview_register_formats(
    selected_extensions: Vec<String>,
) -> Result<ShellPreviewRegistrationSummary, String> {
    let formats = filter_preview_formats(&selected_extensions)?;
    register_shell_previews(&formats)
}
