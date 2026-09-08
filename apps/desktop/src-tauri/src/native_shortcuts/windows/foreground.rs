use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetForegroundWindow, GetWindowThreadProcessId,
};

static MAIN_WINDOW: AtomicIsize = AtomicIsize::new(0);
static OVERLAY_WINDOW: AtomicIsize = AtomicIsize::new(0);

pub(crate) fn register_shell_windows(main: isize, overlay: isize) {
    MAIN_WINDOW.store(main, Ordering::SeqCst);
    OVERLAY_WINDOW.store(overlay, Ordering::SeqCst);
}

pub(super) fn is_shell_foreground() -> bool {
    let root = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) } as isize;
    root != 0
        && (root == MAIN_WINDOW.load(Ordering::SeqCst)
            || root == OVERLAY_WINDOW.load(Ordering::SeqCst))
}

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
