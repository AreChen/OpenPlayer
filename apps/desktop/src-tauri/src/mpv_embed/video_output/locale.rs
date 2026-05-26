#[cfg(unix)]
pub(in crate::mpv_embed) fn prepare_libmpv_numeric_locale() -> Result<(), String> {
    let locale = std::ffi::CString::new("C")
        .map_err(|_| "failed to prepare LC_NUMERIC=C for libmpv".to_string())?;
    // SAFETY: libmpv requires the process C numeric locale to be "C" before
    // mpv_create(). We set only LC_NUMERIC immediately before initializing mpv.
    let result = unsafe { libc::setlocale(libc::LC_NUMERIC, locale.as_ptr()) };
    if result.is_null() {
        Err("failed to set LC_NUMERIC=C before libmpv initialization".to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
pub(in crate::mpv_embed) fn prepare_libmpv_numeric_locale() -> Result<(), String> {
    Ok(())
}
