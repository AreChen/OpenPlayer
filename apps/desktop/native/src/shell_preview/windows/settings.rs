use std::{mem::size_of, ptr::null_mut};

use windows_sys::Win32::System::{
    Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW},
    SystemInformation::{GetVersionExW, OSVERSIONINFOW},
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::{OPENPLAYER_CAPABILITIES_KEY, OPENPLAYER_REGISTERED_APP_NAME, wide_null};

pub(in crate::shell_preview) fn open_default_apps_settings() -> Result<(), String> {
    if supports_app_settings_link() && is_openplayer_registered() {
        let app_settings =
            format!("ms-settings:defaultapps?registeredAppUser={OPENPLAYER_REGISTERED_APP_NAME}");
        if launch_settings(&app_settings) > 32 {
            return Ok(());
        }
    }

    let result = launch_settings("ms-settings:defaultapps");
    if result <= 32 {
        Err(format!(
            "failed to open Windows default apps settings: code {result}"
        ))
    } else {
        Ok(())
    }
}

fn supports_app_settings_link() -> bool {
    // The executable manifest declares Windows 10 and later compatibility.
    let mut version = OSVERSIONINFOW {
        dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    unsafe { GetVersionExW(&mut version) != 0 && version.dwBuildNumber >= 22000 }
}

fn is_openplayer_registered() -> bool {
    let registrations = wide_null("Software\\RegisteredApplications");
    let app = wide_null(OPENPLAYER_REGISTERED_APP_NAME);
    let expected = wide_null(OPENPLAYER_CAPABILITIES_KEY);
    let mut value = vec![0u16; expected.len()];
    let mut length = (value.len() * size_of::<u16>()) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            registrations.as_ptr(),
            app.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            value.as_mut_ptr().cast(),
            &mut length,
        )
    };
    result == 0 && value == expected
}

fn launch_settings(target: &str) -> isize {
    let operation = wide_null("open");
    let target = wide_null(target);
    (unsafe {
        ShellExecuteW(
            null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    }) as isize
}
