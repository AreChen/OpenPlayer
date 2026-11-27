use std::time::{Duration, Instant};

use gpui::{
    Context, KeyDownEvent, Keystroke, MouseButton, MouseDownEvent, MouseMoveEvent, ScrollDelta,
    ScrollEvent, Window,
};
use serde_json::{Value, json};
use windows_sys::Win32::{
    Foundation::POINT,
    UI::{
        Input::KeyboardAndMouse::ReleaseCapture,
        WindowsAndMessaging::{
            GetCursorPos, HTCAPTION, HWND_NOTOPMOST, HWND_TOPMOST, PostMessageW, SWP_NOACTIVATE,
            SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WM_NCLBUTTONDOWN,
        },
    },
};

use super::{PlayerView, hwnd, quick_open::QuickOpenMode};

const DRAG_THRESHOLD: f32 = 4.;
const DOUBLE_CLICK_RADIUS: f32 = 24.;
const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(650);
const VOLUME_STEP: f64 = 0.05;

impl PlayerView {
    pub(super) fn pointer_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activity(cx);
        if self.picker_open {
            return;
        }
        if self.ui.context.is_some() {
            self.send("closeContext", Value::Null, cx);
            return;
        }
        let now = Instant::now();
        let double_click = self.last_stage_click.take().is_some_and(|(at, position)| {
            let distance = event.position - position;
            now.duration_since(at) <= DOUBLE_CLICK_INTERVAL
                && f32::from(distance.x).hypot(f32::from(distance.y)) <= DOUBLE_CLICK_RADIUS
        });
        if double_click && self.ui.media.is_none() {
            self.drag_origin = None;
            self.show_quick_open(QuickOpenMode::Open, cx);
            return;
        }
        if double_click && self.ui.loaded {
            self.drag_origin = None;
            self.send("togglePlayback", Value::Null, cx);
            return;
        }
        self.last_stage_click = Some((now, event.position));
        if !window.is_fullscreen() {
            self.drag_origin = Some(event.position);
        }
    }

    pub(super) fn pointer_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.move_volume_drag(event.position.y, event.pressed_button, cx) {
            cx.stop_propagation();
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) {
            self.drag_origin = None;
            return;
        }
        if let Some(origin) = self.drag_origin {
            let delta = event.position - origin;
            if f32::from(delta.x).hypot(f32::from(delta.y)) >= DRAG_THRESHOLD {
                self.drag_origin = None;
                self.last_stage_click = None;
                begin_native_move(window);
                cx.stop_propagation();
            }
        }
    }

    pub(super) fn wheel(&mut self, event: &ScrollEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.ui.context.is_some()
            || self.ui.settings.is_some()
            || self.ui.stream.is_some()
            || self.quick_open.is_some()
            || self.media_properties.is_some()
        {
            return;
        }
        let vertical = match event.delta {
            ScrollDelta::Lines(lines) => lines.y,
            ScrollDelta::Pixels(pixels) => f32::from(pixels.y),
        };
        if vertical != 0. {
            self.activity(cx);
            self.change_volume(VOLUME_STEP * f64::from(vertical.signum()), true, cx);
            cx.stop_propagation();
        }
    }

    pub(super) fn keyboard(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = &event.keystroke.key;
        if self.media_properties.is_some() {
            cx.stop_propagation();
            if key == "escape" {
                self.close_media_properties(cx);
            }
            return;
        }
        if self.quick_open.is_some() {
            cx.stop_propagation();
            self.quick_open_key(event, window, cx);
            return;
        }
        if let Some(action) = self.ui.recording_shortcut.clone() {
            cx.stop_propagation();
            match key.as_str() {
                "escape" => self.send("recordShortcut", Value::Null, cx),
                "delete" | "backspace" => self.send("assignShortcut", json!([action, null]), cx),
                _ => {
                    if let Some(chord) = shortcut_chord(&event.keystroke) {
                        self.send("assignShortcut", json!([action, chord]), cx);
                    }
                }
            }
            return;
        }
        if key == "escape" {
            if self.edit.is_some() && self.ui.stream.is_none() {
                self.edit = None;
            } else if self.ui.context.is_some() {
                self.send("closeContext", Value::Null, cx);
            } else if self.ui.settings.is_some() {
                self.send("closeSettings", Value::Null, cx);
            } else if self.ui.stream.is_some() {
                self.send("closeNetworkStream", Value::Null, cx);
            } else if window.is_fullscreen() {
                window.toggle_fullscreen();
            }
            self.activity(cx);
            cx.notify();
            return;
        }
        if self.edit.is_some() {
            self.edit_key(event, cx);
            return;
        }
        if self.ui.context.is_some() || self.ui.settings.is_some() || self.ui.stream.is_some() {
            return;
        }
        let Some(chord) = shortcut_chord(&event.keystroke) else {
            return;
        };
        let Some(action) = self
            .ui
            .shortcut_bindings
            .iter()
            .find_map(|(action, binding)| (binding == &chord).then(|| action.clone()))
        else {
            return;
        };
        cx.stop_propagation();
        let repeating = matches!(
            action.as_str(),
            "volumeDown"
                | "volumeUp"
                | "seekBackward"
                | "seekForward"
                | "frameBackward"
                | "frameForward"
        );
        if event.is_held && !repeating {
            return;
        }
        let quiet = self.ui.quiet_keyboard
            && matches!(
                action.as_str(),
                "volumeDown" | "volumeUp" | "seekBackward" | "seekForward"
            );
        if !quiet {
            self.activity(cx);
        }
        match action.as_str() {
            "openMedia" => self.send("openMedia", Value::Null, cx),
            "togglePlayback" => self.send("togglePlayback", Value::Null, cx),
            "restart" => self.send("restartPlayback", Value::Null, cx),
            "togglePlaylist" => self.send("togglePlaylist", Value::Null, cx),
            "seekBackward" => self.send("seekBy", json!(-5.), cx),
            "seekForward" => self.send("seekBy", json!(5.), cx),
            "frameBackward" => self.send("frameBackward", Value::Null, cx),
            "frameForward" => self.send("frameForward", Value::Null, cx),
            "volumeDown" => self.change_volume(-VOLUME_STEP, !quiet, cx),
            "volumeUp" => self.change_volume(VOLUME_STEP, !quiet, cx),
            "toggleFullscreen" => self.toggle_fullscreen(window, cx),
            "toggleAlwaysOnTop" => self.toggle_topmost(window, cx),
            "openSettings" => self.send("openSettings", Value::Null, cx),
            _ => {}
        }
    }

    pub(in crate::windows_gpui) fn toggle_fullscreen(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.toggle_fullscreen();
        self.activity(cx);
        cx.notify();
    }

    pub(super) fn toggle_topmost(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Ok(hwnd) = hwnd(window) else { return };
        let enabled = !self.topmost;
        let insert_after = if enabled {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        let applied = unsafe {
            SetWindowPos(
                hwnd as _,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
            )
        };
        if applied != 0 {
            self.topmost = enabled;
            let label = self.ui.label(if enabled {
                "status.alwaysOnTopEnabled"
            } else {
                "status.alwaysOnTopDisabled"
            });
            self.show_indicator("pin", label, cx);
        }
    }

    fn change_volume(&mut self, delta: f64, feedback: bool, cx: &mut Context<Self>) {
        let next = (self.ui.volume + delta).clamp(0., 1.);
        self.send("volume", json!(next), cx);
        if feedback {
            self.show_volume(cx);
        }
    }
}

fn begin_native_move(window: &Window) {
    let Ok(hwnd) = hwnd(window) else { return };
    let mut cursor = POINT::default();
    unsafe {
        if GetCursorPos(&mut cursor) == 0 {
            return;
        }
        // GPUI Fast's Win32 backend delegates caption dragging to DefWindowProc.
        // Post instead of sending so the native moving loop starts after GPUI returns.
        ReleaseCapture();
        let packed = (u32::from(cursor.x as u16) | u32::from(cursor.y as u16) << 16) as isize;
        PostMessageW(hwnd as _, WM_NCLBUTTONDOWN, HTCAPTION as usize, packed);
    }
}

fn shortcut_chord(keystroke: &Keystroke) -> Option<String> {
    let raw = keystroke.key.as_str();
    let key = match raw {
        "alt" | "ctrl" | "control" | "meta" | "shift" | "win" => return None,
        " " | "space" => "Space".into(),
        "escape" => "Escape".into(),
        "left" => "ArrowLeft".into(),
        "right" => "ArrowRight".into(),
        "up" => "ArrowUp".into(),
        "down" => "ArrowDown".into(),
        "enter" => "Enter".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Delete".into(),
        "tab" => "Tab".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        _ if raw.len() == 1
            || raw
                .strip_prefix('f')
                .is_some_and(|rest| rest.parse::<u8>().is_ok()) =>
        {
            raw.to_ascii_uppercase()
        }
        _ => raw.to_owned(),
    };
    let modifiers = keystroke.modifiers;
    let mut parts = Vec::with_capacity(5);
    for (pressed, name) in [
        (modifiers.control, "Ctrl"),
        (modifiers.platform, "Meta"),
        (modifiers.alt, "Alt"),
        (modifiers.shift, "Shift"),
    ] {
        if pressed {
            parts.push(name);
        }
    }
    parts.push(&key);
    Some(parts.join("+"))
}
