use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

pub(super) fn is_openplayer_foreground() -> bool {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_null() {
        return false;
    }

    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(foreground, &mut process_id);
    }
    process_id == std::process::id()
}
