use std::ptr::{null, null_mut};

use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_DWORD, REG_NONE, REG_SZ, RegCloseKey, RegCreateKeyW,
    RegSetValueExW,
};

use super::wide_null;

pub(super) fn write_reg_string(subkey: &str, value_name: &str, value: &str) -> Result<(), String> {
    let key = create_current_user_key(subkey)?;
    let name_w = wide_null(value_name);
    let value_name_ptr = if value_name.is_empty() {
        null()
    } else {
        name_w.as_ptr()
    };
    let value_w = wide_null(value);
    let status = unsafe {
        RegSetValueExW(
            key,
            value_name_ptr,
            0,
            REG_SZ,
            value_w.as_ptr().cast::<u8>(),
            (value_w.len() * 2) as u32,
        )
    };
    close_key(key);

    if status != 0 {
        return Err(reg_write_error(subkey, value_name, status));
    }

    Ok(())
}

pub(super) fn write_reg_none(subkey: &str, value_name: &str) -> Result<(), String> {
    let key = create_current_user_key(subkey)?;
    let name_w = wide_null(value_name);
    let status = unsafe { RegSetValueExW(key, name_w.as_ptr(), 0, REG_NONE, null(), 0) };
    close_key(key);

    if status != 0 {
        return Err(reg_write_error(subkey, value_name, status));
    }

    Ok(())
}

pub(super) fn write_reg_dword(subkey: &str, value_name: &str, value: u32) -> Result<(), String> {
    let key = create_current_user_key(subkey)?;
    let name_w = wide_null(value_name);
    let value_bytes = value.to_le_bytes();
    let status = unsafe {
        RegSetValueExW(
            key,
            name_w.as_ptr(),
            0,
            REG_DWORD,
            value_bytes.as_ptr(),
            value_bytes.len() as u32,
        )
    };
    close_key(key);

    if status != 0 {
        return Err(reg_write_error(subkey, value_name, status));
    }

    Ok(())
}

pub(super) fn notify_shell_association_changed() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_FLUSH, SHChangeNotify};

    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_FLUSH, null(), null());
    }
}

fn create_current_user_key(subkey: &str) -> Result<HKEY, String> {
    let mut key: HKEY = null_mut();
    let subkey_w = wide_null(subkey);
    let status = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr(), &mut key) };
    if status != 0 {
        return Err(format!(
            "failed to open registry key {subkey}: code {status}"
        ));
    }

    Ok(key)
}

fn close_key(key: HKEY) {
    unsafe {
        RegCloseKey(key);
    }
}

fn reg_write_error(subkey: &str, value_name: &str, status: u32) -> String {
    format!("failed to write registry value {subkey}\\{value_name}: code {status}")
}
