//! Probe-only CUDA enumeration; never creates a context or selects another GPU.
use std::ffi::{CStr, c_char};
use windows_sys::Win32::Foundation::{FreeLibrary, HMODULE};
use windows_sys::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};

type Procedure = unsafe extern "system" fn() -> isize;
type CuInit = unsafe extern "system" fn(u32) -> i32;
type CuDeviceGetCount = unsafe extern "system" fn(*mut i32) -> i32;
type CuDeviceGet = unsafe extern "system" fn(*mut i32, i32) -> i32;
type CuDeviceGetLuid = unsafe extern "system" fn(*mut c_char, *mut u32, i32) -> i32;

struct Driver(HMODULE);

impl Driver {
    fn load() -> Result<Self, String> {
        let name: Vec<u16> = "nvcuda.dll\0".encode_utf16().collect();
        // The terminated filename remains valid for this call; only System32 is searched.
        let module = unsafe {
            LoadLibraryExW(
                name.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if module.is_null() {
            return Err(format!(
                "CUDA driver unavailable: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self(module))
    }

    fn symbol(&self, name: &CStr) -> Result<Procedure, String> {
        // self owns a live module and CStr supplies a terminated export name.
        unsafe { GetProcAddress(self.0, name.as_ptr().cast()) }.ok_or_else(|| {
            format!(
                "CUDA export {} unavailable: {}",
                name.to_string_lossy(),
                std::io::Error::last_os_error()
            )
        })
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        // The non-null LoadLibraryExW reference is owned exclusively by this guard.
        unsafe { FreeLibrary(self.0) };
    }
}

fn check(operation: &str, status: i32) -> Result<(), String> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{operation} failed with CUDA status {status}"))
    }
}

pub(super) fn device_for_luid(expected: u64) -> Result<i32, String> {
    let driver = Driver::load()?;
    // These exact CUDA exports use CUDAAPI (Windows system ABI). The driver
    // guard outlives every function pointer and every invocation below.
    let (init, count_devices, get_device, get_luid) = unsafe {
        (
            std::mem::transmute::<Procedure, CuInit>(driver.symbol(c"cuInit")?),
            std::mem::transmute::<Procedure, CuDeviceGetCount>(driver.symbol(c"cuDeviceGetCount")?),
            std::mem::transmute::<Procedure, CuDeviceGet>(driver.symbol(c"cuDeviceGet")?),
            std::mem::transmute::<Procedure, CuDeviceGetLuid>(driver.symbol(c"cuDeviceGetLuid")?),
        )
    };
    check("cuInit", unsafe { init(0) })?;
    let mut count = 0;
    check("cuDeviceGetCount", unsafe { count_devices(&mut count) })?;
    if !(1..=32).contains(&count) {
        return Err(format!("invalid CUDA device count: {count}"));
    }

    let mut identities = Vec::with_capacity(count as usize);
    for ordinal in 0..count {
        let mut device = 0;
        let mut luid = [0u8; 8];
        let mut node_mask = 0;
        // All output buffers are writable and sized for the CUDA driver ABI.
        check("cuDeviceGet", unsafe { get_device(&mut device, ordinal) })?;
        check("cuDeviceGetLuid", unsafe {
            get_luid(luid.as_mut_ptr().cast(), &mut node_mask, device)
        })?;
        identities.push((u64::from_le_bytes(luid), node_mask));
    }
    // Finish enumeration before matching so a later failure or duplicate cannot
    // turn an earlier match into a successful selection.
    match_identity(expected, &identities)
}

fn match_identity(expected: u64, identities: &[(u64, u32)]) -> Result<i32, String> {
    let mut matched = None;
    for (ordinal, &(luid, node_mask)) in identities.iter().enumerate() {
        if luid != expected {
            continue;
        }
        if node_mask != 1 {
            return Err(format!(
                "CUDA LUID {expected:016x} has unsupported node mask {node_mask}; expected 1"
            ));
        }
        if matched.is_some() {
            return Err(format!("duplicate CUDA LUID {expected:016x}"));
        }
        matched = Some(i32::try_from(ordinal).map_err(|_| "CUDA ordinal exceeds i32".to_string())?);
    }
    matched.ok_or_else(|| {
        format!("selected LUID {expected:016x} has no matching CUDA decoder; no fallback")
    })
}

#[cfg(test)]
mod tests {
    use super::match_identity;

    #[test]
    fn returns_exact_matching_enumeration_ordinal() {
        let expected = 0xfedc_ba98_7654_3210;
        let identities = [(0x7654_3210, 1), (expected ^ (1 << 32), 1), (expected, 1)];
        assert_eq!(match_identity(expected, &identities), Ok(2));
        assert_eq!(match_identity(expected, &[(expected, 1)]), Ok(0));
    }

    #[test]
    fn rejects_duplicate_matching_luids() {
        assert!(match_identity(7, &[(7, 1), (8, 1), (7, 1)]).is_err());
    }

    #[test]
    fn rejects_missing_luid_without_falling_back() {
        assert!(match_identity(7, &[]).is_err());
        assert!(match_identity(7, &[(8, 1), (9, 1)]).is_err());
    }

    #[test]
    fn rejects_zero_non_primary_and_multi_node_masks() {
        for mask in [0, 2, 3, u32::MAX] {
            assert!(match_identity(7, &[(7, mask)]).is_err());
            assert!(match_identity(7, &[(7, 1), (7, mask)]).is_err());
        }
    }
}
