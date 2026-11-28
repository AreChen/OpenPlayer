mod formats;
mod windows;

use formats::{PREVIEW_FORMATS, ShellPreviewFormat, filter_preview_formats};
pub use formats::{ShellPreviewFormatInfo, ShellPreviewRegistrationSummary};
#[cfg(windows)]
pub(crate) use windows::{ShellPreviewDefaultStatus, shell_preview_default_status};
use windows::{open_default_apps_settings, register_shell_previews};

pub fn shell_preview_formats() -> Vec<ShellPreviewFormatInfo> {
    PREVIEW_FORMATS
        .iter()
        .copied()
        .map(ShellPreviewFormat::info)
        .collect()
}

pub fn shell_preview_open_default_apps_settings() -> Result<(), String> {
    open_default_apps_settings()
}

pub fn shell_preview_register_formats(
    selected_extensions: Vec<String>,
) -> Result<ShellPreviewRegistrationSummary, String> {
    let formats = filter_preview_formats(&selected_extensions)?;
    register_shell_previews(&formats)
}
