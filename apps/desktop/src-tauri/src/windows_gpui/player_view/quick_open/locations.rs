use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf, ptr};

use windows_sys::{
    Win32::{
        Storage::FileSystem::GetLogicalDrives,
        System::Com::CoTaskMemFree,
        UI::Shell::{
            FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Downloads, FOLDERID_Profile,
            FOLDERID_Videos, KF_FLAG_DONT_VERIFY, SHGetKnownFolderPath,
        },
    },
    core::GUID,
};

#[derive(Clone, Copy)]
pub(super) enum QuickLocation {
    Home,
    Desktop,
    Documents,
    Downloads,
    Videos,
}

impl QuickLocation {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Home => "media.quickOpen.home",
            Self::Desktop => "media.quickOpen.desktop",
            Self::Documents => "media.quickOpen.documents",
            Self::Downloads => "media.quickOpen.downloads",
            Self::Videos => "media.quickOpen.videos",
        }
    }
}

pub(super) fn home_directory() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub(super) fn system_locations() -> (Vec<(QuickLocation, PathBuf)>, Vec<PathBuf>) {
    use QuickLocation::{Desktop, Documents, Downloads, Home, Videos};
    let places = [
        (Home, FOLDERID_Profile),
        (Desktop, FOLDERID_Desktop),
        (Documents, FOLDERID_Documents),
        (Downloads, FOLDERID_Downloads),
        (Videos, FOLDERID_Videos),
    ]
    .into_iter()
    .filter_map(|(location, id)| known_folder(id).map(|path| (location, path)))
    .collect();
    // This reads the mapped drive bitset without touching each drive's filesystem.
    let mask = unsafe { GetLogicalDrives() };
    let drives = (0..26)
        .filter(|index| mask & (1 << index) != 0)
        .map(|index| PathBuf::from(format!("{}:\\", char::from(b'A' + index))))
        .collect();
    (places, drives)
}

fn known_folder(id: GUID) -> Option<PathBuf> {
    let mut raw = ptr::null_mut();
    // The shell returns a NUL-terminated path allocated with CoTaskMemAlloc.
    unsafe {
        let result =
            SHGetKnownFolderPath(&id, KF_FLAG_DONT_VERIFY as u32, ptr::null_mut(), &mut raw);
        let path = (result >= 0 && !raw.is_null()).then(|| {
            let length = (0..).take_while(|&index| *raw.add(index) != 0).count();
            PathBuf::from(OsString::from_wide(std::slice::from_raw_parts(raw, length)))
        });
        CoTaskMemFree(raw.cast());
        path
    }
}
