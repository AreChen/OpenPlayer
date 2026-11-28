use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::OsStringExt,
    path::Path,
    ptr::{null, null_mut},
};

use windows_sys::{
    Win32::{
        Foundation::{ERROR_NO_ASSOCIATION, S_FALSE, S_OK},
        UI::Shell::{
            ASSOCF_NOTRUNCATE, ASSOCSTR, ASSOCSTR_EXECUTABLE, ASSOCSTR_PROGID, AssocQueryStringW,
        },
    },
    core::HRESULT,
};

use crate::media_paths::SUPPORTED_MEDIA_EXTENSIONS;

use super::{OPENPLAYER_PROG_ID, wide_null};

const NO_ASSOCIATION: HRESULT = (0x8007_0000 | ERROR_NO_ASSOCIATION) as HRESULT;
const PLAYER_EXECUTABLE: &str = "openplayer-desktop.exe";
const OPENPLAYER_LEGACY_PROG_ID: &str = "OpenPlayer Media";
const OPENPLAYER_DIRECT_PROG_ID: &str = "Applications\\openplayer-desktop.exe";

#[derive(Debug, Default)]
pub(crate) struct ShellPreviewDefaultStatus {
    pub(crate) total_count: usize,
    pub(crate) openplayer_count: usize,
    pub(crate) other_count: usize,
    pub(crate) unassigned_count: usize,
    pub(crate) unknown_count: usize,
}

/// Read the effective Windows associations advertised by the OpenPlayer installer.
pub(crate) fn shell_preview_default_status() -> ShellPreviewDefaultStatus {
    let mut status = ShellPreviewDefaultStatus {
        total_count: SUPPORTED_MEDIA_EXTENSIONS.len(),
        ..Default::default()
    };
    for extension in SUPPORTED_MEDIA_EXTENSIONS {
        let extension = wide_null(&format!(".{extension}"));
        match query_association(&extension, ASSOCSTR_PROGID) {
            Ok(prog_id) if prog_id.eq_ignore_ascii_case(OsStr::new("Unknown")) => {
                status.unassigned_count += 1;
            }
            Ok(prog_id) if is_openplayer_prog_id(&prog_id) => {
                match query_association(&extension, ASSOCSTR_EXECUTABLE) {
                    Ok(executable) => {
                        let path = Path::new(&executable);
                        if path.file_name().is_some_and(|name| {
                            name.eq_ignore_ascii_case(OsStr::new(PLAYER_EXECUTABLE))
                        }) && path.is_file()
                        {
                            status.openplayer_count += 1;
                        } else {
                            status.unknown_count += 1;
                        }
                    }
                    Err(NO_ASSOCIATION) => status.unassigned_count += 1,
                    Err(_) => status.unknown_count += 1,
                }
            }
            Ok(_) => status.other_count += 1,
            Err(NO_ASSOCIATION) => status.unassigned_count += 1,
            Err(_) => status.unknown_count += 1,
        }
    }
    status
}

fn is_openplayer_prog_id(prog_id: &OsStr) -> bool {
    [
        OPENPLAYER_PROG_ID,
        OPENPLAYER_LEGACY_PROG_ID,
        OPENPLAYER_DIRECT_PROG_ID,
    ]
    .iter()
    .any(|known| prog_id.eq_ignore_ascii_case(OsStr::new(known)))
}

fn query_association(extension: &[u16], property: ASSOCSTR) -> Result<OsString, HRESULT> {
    let mut length = 0;
    let result = unsafe {
        AssocQueryStringW(
            ASSOCF_NOTRUNCATE,
            property,
            extension.as_ptr(),
            null(),
            null_mut(),
            &mut length,
        )
    };
    if result != S_FALSE || length == 0 {
        return Err(result);
    }
    let mut buffer = vec![0u16; length as usize];
    let result = unsafe {
        AssocQueryStringW(
            ASSOCF_NOTRUNCATE,
            property,
            extension.as_ptr(),
            null(),
            buffer.as_mut_ptr(),
            &mut length,
        )
    };
    if result != S_OK {
        return Err(result);
    }
    let value = buffer.split(|&unit| unit == 0).next().unwrap_or_default();
    if value.is_empty() {
        return Err(NO_ASSOCIATION);
    }
    Ok(OsString::from_wide(value))
}
