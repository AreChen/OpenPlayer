use std::{
    collections::{HashMap, HashSet},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

mod foreground;
mod keyboard;

pub(super) use foreground::register_shell_windows;
use foreground::{is_openplayer_foreground, is_shell_foreground};
use keyboard::native_shortcut_chord;
use tauri::{AppHandle, Emitter};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_F4, VK_LWIN, VK_RWIN, VK_SHIFT,
    },
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, LLKHF_ALTDOWN, MSG, SetWindowsHookExW,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    },
};

const NATIVE_SHORTCUT_EVENT: &str = "openplayer-native-shortcut";

struct NativeShortcutState {
    app: AppHandle,
    shortcuts: Mutex<HashMap<String, String>>,
    enabled: AtomicBool,
    pressed_keys: Mutex<HashSet<u32>>,
    hook: Mutex<Option<usize>>,
}

static NATIVE_SHORTCUT_STATE: OnceLock<NativeShortcutState> = OnceLock::new();

pub(super) fn install_native_shortcut_hook(app: AppHandle) {
    let _ = NATIVE_SHORTCUT_STATE.set(NativeShortcutState {
        app,
        shortcuts: Mutex::new(HashMap::new()),
        enabled: AtomicBool::new(true),
        pressed_keys: Mutex::new(HashSet::new()),
        hook: Mutex::new(None),
    });

    thread::spawn(|| unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(native_shortcut_keyboard_proc),
            module,
            0,
        );
        if hook.is_null() {
            return;
        }

        if let Some(state) = NATIVE_SHORTCUT_STATE.get()
            && let Ok(mut stored_hook) = state.hook.lock()
        {
            *stored_hook = Some(hook as usize);
        }

        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {}
    });
}

pub(super) fn update_native_shortcuts(bindings: HashMap<String, Option<String>>) {
    let Some(state) = NATIVE_SHORTCUT_STATE.get() else {
        return;
    };
    let Ok(mut shortcuts) = state.shortcuts.lock() else {
        return;
    };

    shortcuts.clear();
    for (action, chord) in bindings {
        if let Some(chord) = chord {
            shortcuts.insert(chord, action);
        }
    }
}

pub(super) fn set_native_shortcuts_enabled(enabled: bool) {
    if let Some(state) = NATIVE_SHORTCUT_STATE.get() {
        state.enabled.store(enabled, Ordering::SeqCst);
    }
}

pub(super) fn capture_recovery_available() -> bool {
    NATIVE_SHORTCUT_STATE
        .get()
        .and_then(|state| state.hook.lock().ok())
        .is_some_and(|hook| hook.is_some())
}

pub(crate) fn recover_capture_on_escape(app: &AppHandle, vk_code: u32, foreground: bool) -> bool {
    if foreground && vk_code == u32::from(VK_ESCAPE) && crate::window::capture_mode_active(app) {
        crate::window::restore_capture_controls(app);
        true
    } else {
        false
    }
}

unsafe extern "system" fn native_shortcut_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    }

    let message = wparam as u32;
    let is_key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    let is_key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
    if !is_key_down && !is_key_up {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    }

    let Some(state) = NATIVE_SHORTCUT_STATE.get() else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    };
    let key = unsafe { *(lparam as *const KBDLLHOOKSTRUCT) };
    if is_key_up && let Ok(mut pressed_keys) = state.pressed_keys.lock() {
        pressed_keys.remove(&key.vkCode);
    }

    let foreground = is_openplayer_foreground();
    // File pickers and other native dialogs keep their normal Escape/Alt+F4 behavior.
    let shell_foreground = foreground && is_shell_foreground();
    // Native video/WebView focus can consume the default system-close route after transitions.
    // Keep Alt+F4 on the same coordinated close path as our titlebar, including capture mode.
    if is_key_down
        && shell_foreground
        && key.vkCode == u32::from(VK_F4)
        && key.flags & LLKHF_ALTDOWN != 0
        && [VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN]
            .iter()
            .all(|key| unsafe { GetAsyncKeyState(i32::from(*key)) } >= 0)
    {
        crate::window::request_native_close(&state.app);
        return 1;
    }
    // Recovery is native and must work even when a hidden WebView disabled ordinary shortcuts.
    if is_key_down
        && key.vkCode == u32::from(VK_ESCAPE)
        && native_shortcut_chord(key.vkCode).as_deref() == Some("Escape")
        && recover_capture_on_escape(&state.app, key.vkCode, shell_foreground)
    {
        return 1;
    }
    if !state.enabled.load(Ordering::SeqCst) || !foreground {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    }

    let Some(chord) = native_shortcut_chord(key.vkCode) else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    };
    let action = state
        .shortcuts
        .lock()
        .ok()
        .and_then(|shortcuts| shortcuts.get(&chord).cloned());
    let Some(action) = action else {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), ncode, wparam, lparam) };
    };

    if is_key_up {
        return 1;
    }

    let first_press = state
        .pressed_keys
        .lock()
        .map(|mut pressed_keys| pressed_keys.insert(key.vkCode))
        .unwrap_or(true);
    if first_press {
        let _ = state.app.emit_to("overlay", NATIVE_SHORTCUT_EVENT, action);
    }

    1
}
