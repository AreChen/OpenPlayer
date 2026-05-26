use super::super::*;

pub(in crate::mpv_embed) fn mpv_error_message(code: i32) -> String {
    let message = unsafe { libmpv2_sys::mpv_error_string(code) };
    if message.is_null() {
        return code.to_string();
    }

    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}
