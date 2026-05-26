use std::ptr::null_mut;

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::wide_null;

pub(in crate::shell_preview) fn open_default_apps_settings() -> Result<(), String> {
    let operation = wide_null("open");
    let target = wide_null("ms-settings:defaultapps");
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    } as isize;

    if result <= 32 {
        Err(format!(
            "failed to open Windows default apps settings: code {result}"
        ))
    } else {
        Ok(())
    }
}
