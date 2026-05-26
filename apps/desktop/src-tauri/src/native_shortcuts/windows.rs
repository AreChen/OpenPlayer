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

use foreground::is_openplayer_foreground;
use keyboard::native_shortcut_chord;
use tauri::{AppHandle, Emitter};
use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, KBDLLHOOKSTRUCT, MSG, SetWindowsHookExW, WH_KEYBOARD_LL,
        WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
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

    if !state.enabled.load(Ordering::SeqCst) || !is_openplayer_foreground() {
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
