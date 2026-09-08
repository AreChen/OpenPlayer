//! Verified, process-owned runtime copies. Currently exercised by developer smoke
//! tests only; not a plugin command or an authorization boundary for native code.
mod cache;
mod inventory;
#[cfg(test)]
mod tests;

use std::{
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};
use windows_sys::Win32::System::LibraryLoader::{
    GetModuleHandleW, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LOAD_LIBRARY_SEARCH_SYSTEM32,
    LoadLibraryExW,
};

static RESIDENT: LazyLock<Mutex<Option<Resident>>> = LazyLock::new(|| Mutex::new(None));
const VSSCRIPT: &str = "runtime/Lib/site-packages/vapoursynth/vsscript.dll";

struct Resident {
    copy: cache::RuntimeCopy,
    // Deliberately never FreeLibrary: embedded Python and VSScript have static
    // process state. The OS releases the module and lease at process exit.
    _library: isize,
}

pub(crate) fn pin_vsscript(source: &Path, cache_root: &Path) -> Result<PathBuf, String> {
    if std::env::var_os("VSSCRIPT_PATH").is_some() {
        return Err("external VSSCRIPT_PATH conflicts with the host-owned runtime".into());
    }
    let inventory = inventory::Inventory::read(source)?;
    let mut resident = RESIDENT
        .lock()
        .map_err(|_| "native runtime registry unavailable")?;
    if let Some(loaded) = resident.as_ref() {
        if loaded.copy.digest != inventory.digest {
            return Err("a different video runtime is already resident; restart required".into());
        }
        return Ok(loaded.copy.path.clone());
    }
    for name in [
        "vsscript.dll",
        "libvapoursynth.dll",
        "python3.dll",
        "python312.dll",
    ] {
        if !unsafe { GetModuleHandleW(wide(Path::new(name)).as_ptr()) }.is_null() {
            return Err("an unmanaged Python or VSScript runtime is already loaded".into());
        }
    }
    for required in [
        VSSCRIPT,
        "runtime/python.exe",
        "runtime/python3.dll",
        "runtime/python312.dll",
        "runtime/python312._pth",
    ] {
        if !inventory.files.contains_key(required) {
            return Err(format!("runtime inventory is missing {required}"));
        }
    }
    let mut copy = cache::RuntimeCopy::prepare(source, cache_root, &inventory)?;
    let library = unsafe {
        LoadLibraryExW(
            wide(&copy.path.join(VSSCRIPT)).as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32,
        )
    };
    if library.is_null() {
        return Err(format!(
            "native runtime load failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    copy.retain_until_exit();
    let path = copy.path.clone();
    *resident = Some(Resident {
        copy,
        _library: library as isize,
    });
    Ok(path)
}

fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
