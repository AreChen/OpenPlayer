use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_LWIN,
    VK_MENU, VK_OEM_5, VK_OEM_COMMA, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_UP,
};

pub(super) fn native_shortcut_chord(vk_code: u32) -> Option<String> {
    let key = native_shortcut_key(vk_code)?;
    let mut parts = Vec::new();

    if native_modifier_down(VK_CONTROL) {
        parts.push("Ctrl");
    }
    if native_modifier_down(VK_LWIN) || native_modifier_down(VK_RWIN) {
        parts.push("Meta");
    }
    if native_modifier_down(VK_MENU) {
        parts.push("Alt");
    }
    if native_modifier_down(VK_SHIFT) {
        parts.push("Shift");
    }

    parts.push(key);
    Some(parts.join("+"))
}

fn native_modifier_down(vkey: u16) -> bool {
    unsafe { GetAsyncKeyState(vkey as i32) < 0 }
}

fn native_shortcut_key(vk_code: u32) -> Option<&'static str> {
    match vk_code {
        0x30 => Some("0"),
        0x31 => Some("1"),
        0x32 => Some("2"),
        0x33 => Some("3"),
        0x34 => Some("4"),
        0x35 => Some("5"),
        0x36 => Some("6"),
        0x37 => Some("7"),
        0x38 => Some("8"),
        0x39 => Some("9"),
        0x41 => Some("A"),
        0x42 => Some("B"),
        0x43 => Some("C"),
        0x44 => Some("D"),
        0x45 => Some("E"),
        0x46 => Some("F"),
        0x47 => Some("G"),
        0x48 => Some("H"),
        0x49 => Some("I"),
        0x4A => Some("J"),
        0x4B => Some("K"),
        0x4C => Some("L"),
        0x4D => Some("M"),
        0x4E => Some("N"),
        0x4F => Some("O"),
        0x50 => Some("P"),
        0x51 => Some("Q"),
        0x52 => Some("R"),
        0x53 => Some("S"),
        0x54 => Some("T"),
        0x55 => Some("U"),
        0x56 => Some("V"),
        0x57 => Some("W"),
        0x58 => Some("X"),
        0x59 => Some("Y"),
        value if value == VK_BACK as u32 => Some("Backspace"),
        value if value == VK_DELETE as u32 => Some("Delete"),
        value if value == VK_DOWN as u32 => Some("ArrowDown"),
        value if value == VK_ESCAPE as u32 => Some("Escape"),
        value if value == VK_LEFT as u32 => Some("ArrowLeft"),
        value if value == VK_OEM_5 as u32 => Some("\\"),
        value if value == VK_OEM_COMMA as u32 => Some(","),
        value if value == VK_RETURN as u32 => Some("Enter"),
        value if value == VK_RIGHT as u32 => Some("ArrowRight"),
        value if value == VK_SPACE as u32 => Some("Space"),
        value if value == VK_UP as u32 => Some("ArrowUp"),
        _ => None,
    }
}
