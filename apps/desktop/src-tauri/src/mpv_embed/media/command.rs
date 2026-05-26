use super::*;

pub(super) fn mpv_command_async(
    mpv: &libmpv2::Mpv,
    request_id: u64,
    name: &str,
    args: &[&str],
) -> Result<(), String> {
    let mut cstr_args = Vec::with_capacity(args.len() + 1);
    cstr_args
        .push(CString::new(name).map_err(|error| format!("mpv command name failed: {error}"))?);

    for arg in args {
        cstr_args.push(
            CString::new(*arg).map_err(|error| format!("mpv command argument failed: {error}"))?,
        );
    }

    let mut ptrs: Vec<_> = cstr_args.iter().map(|cstr| cstr.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    let result =
        unsafe { libmpv2_sys::mpv_command_async(mpv.ctx.as_ptr(), request_id, ptrs.as_mut_ptr()) };
    if result < 0 {
        Err(format!(
            "mpv {name} async failed: {}",
            mpv_error_message(result)
        ))
    } else {
        Ok(())
    }
}
