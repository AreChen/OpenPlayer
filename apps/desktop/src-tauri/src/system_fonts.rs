#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr::null_mut};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
    System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumValueW,
        RegOpenKeyExW,
    },
};

#[cfg(test)]
mod tests;

#[tauri::command]
pub(crate) fn system_font_families() -> Vec<String> {
    system_font_families_impl()
}
#[cfg(windows)]
fn system_font_families_impl() -> Vec<String> {
    let mut fonts = Vec::new();
    collect_windows_registry_fonts(
        HKEY_LOCAL_MACHINE,
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts",
        &mut fonts,
    );
    collect_windows_registry_fonts(
        HKEY_CURRENT_USER,
        "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts",
        &mut fonts,
    );
    normalize_font_family_list(fonts)
}

#[cfg(not(windows))]
fn system_font_families_impl() -> Vec<String> {
    default_font_families()
}

#[cfg(windows)]
fn collect_windows_registry_fonts(root: HKEY, subkey: &str, fonts: &mut Vec<String>) {
    let mut key: HKEY = null_mut();
    let subkey = wide_null(subkey);
    let status = unsafe { RegOpenKeyExW(root, subkey.as_ptr(), 0, KEY_READ, &mut key) };
    if status != ERROR_SUCCESS {
        return;
    }

    let mut index = 0;
    loop {
        let mut name = [0u16; 512];
        let mut name_len = name.len() as u32;
        let status = unsafe {
            RegEnumValueW(
                key,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        if status != ERROR_SUCCESS {
            break;
        }
        if let Some(font) =
            registry_font_name_to_family(&String::from_utf16_lossy(&name[..name_len as usize]))
        {
            fonts.push(font);
        }
        index += 1;
    }

    unsafe {
        RegCloseKey(key);
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn registry_font_name_to_family(name: &str) -> Option<String> {
    let trimmed = name.trim().trim_start_matches('@').trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut family = trimmed
        .split_once(" (")
        .map(|(family, _)| family)
        .unwrap_or(trimmed)
        .trim()
        .to_string();
    for suffix in [
        " Bold Italic",
        " Bold Oblique",
        " SemiBold Italic",
        " Semibold Italic",
        " ExtraBold Italic",
        " Light Italic",
        " Medium Italic",
        " Regular",
        " Bold",
        " Italic",
        " Oblique",
        " SemiBold",
        " Semibold",
        " ExtraBold",
        " Medium",
        " Light",
        " Black",
    ] {
        if family.len() > suffix.len()
            && family
                .to_ascii_lowercase()
                .ends_with(&suffix.to_ascii_lowercase())
        {
            family.truncate(family.len() - suffix.len());
            break;
        }
    }
    let family = family.trim().to_string();
    (!family.is_empty()).then_some(family)
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
fn normalize_font_family_list(fonts: Vec<String>) -> Vec<String> {
    let mut fonts = if fonts.is_empty() {
        default_font_families()
    } else {
        fonts
    };
    fonts.sort_by(|left, right| {
        left.to_ascii_lowercase()
            .cmp(&right.to_ascii_lowercase())
            .then_with(|| left.cmp(right))
    });
    fonts.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    fonts
}

fn default_font_families() -> Vec<String> {
    [
        "Arial",
        "Microsoft YaHei",
        "Segoe UI",
        "SimHei",
        "sans-serif",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}
