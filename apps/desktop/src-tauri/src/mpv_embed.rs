use std::{path::PathBuf, sync::Mutex};

use libmpv2::{events::Event, mpv_end_file_reason};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Serialize;
use tauri::{State, Window};
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetClientRect, MoveWindow, SW_SHOW, SetParent, ShowWindow,
        WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
    },
};

const VIDEO_HOST_TOP_RESERVE: i32 = 0;
const VIDEO_HOST_BOTTOM_RESERVE: i32 = 0;
const END_OF_MEDIA_SNAP_TOLERANCE_SECONDS: f64 = 0.5;

#[derive(Debug, PartialEq, Eq)]
struct VideoHostRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Default)]
pub struct MpvEmbedState {
    player: Mutex<Option<MpvEmbedPlayer>>,
}

struct MpvEmbedPlayer {
    mpv: libmpv2::Mpv,
    host: MpvVideoHost,
    path: String,
    volume: f64,
    ended: bool,
}

struct MpvVideoHost {
    parent_hwnd: isize,
    hwnd: isize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvEmbedSnapshot {
    path: String,
    hwnd: i64,
    status: String,
    ended: bool,
    paused: bool,
    position: f64,
    duration: f64,
    volume: f64,
}

#[tauri::command]
#[allow(dead_code)]
pub fn mpv_embed_open_path(
    window: Window,
    state: State<'_, MpvEmbedState>,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    open_path_for_window(&window, state.inner(), path)
}

pub fn open_path_for_window(
    window: &impl HasWindowHandle,
    state: &MpvEmbedState,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    let path = validate_media_path(&path)?;
    let parent_hwnd = window_hwnd(window)?;
    let host = MpvVideoHost::new(parent_hwnd)?;
    let hwnd = host.hwnd as i64;
    let mpv = create_embed_player(hwnd)?;
    let path_text = path.to_string_lossy().to_string();

    mpv.command("loadfile", &[&path_text, "replace"])
        .map_err(|error| format!("mpv loadfile failed: {error}"))?;

    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    let mut next_player = MpvEmbedPlayer {
        mpv,
        host,
        path: path_text,
        volume: 82.0,
        ended: false,
    };
    let snapshot = next_player.snapshot(hwnd, "playing");
    *player = Some(next_player);

    Ok(snapshot)
}

#[tauri::command]
pub fn mpv_embed_play(state: State<'_, MpvEmbedState>) -> Result<MpvEmbedSnapshot, String> {
    with_player(&state, |player| {
        player
            .mpv
            .set_property("pause", false)
            .map_err(|error| format!("mpv play failed: {error}"))?;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_pause(state: State<'_, MpvEmbedState>) -> Result<MpvEmbedSnapshot, String> {
    with_player(&state, |player| {
        player
            .mpv
            .set_property("pause", true)
            .map_err(|error| format!("mpv pause failed: {error}"))?;
        Ok(player.snapshot(0, "paused"))
    })
}

#[tauri::command]
pub fn mpv_embed_seek(
    state: State<'_, MpvEmbedState>,
    position: f64,
) -> Result<MpvEmbedSnapshot, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("invalid mpv seek target".to_string());
    }

    with_player(&state, |player| {
        player
            .mpv
            .command("seek", &[&position.to_string(), "absolute"])
            .map_err(|error| format!("mpv seek failed: {error}"))?;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_set_volume(
    state: State<'_, MpvEmbedState>,
    volume: f64,
) -> Result<MpvEmbedSnapshot, String> {
    if !volume.is_finite() {
        return Err("invalid mpv volume".to_string());
    }

    let volume = volume.clamp(0.0, 100.0);
    with_player(&state, |player| {
        player
            .mpv
            .set_property("volume", volume)
            .map_err(|error| format!("mpv volume failed: {error}"))?;
        player.volume = volume;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_snapshot(
    state: State<'_, MpvEmbedState>,
) -> Result<Option<MpvEmbedSnapshot>, String> {
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;

    Ok(player.as_mut().map(|player| player.snapshot(0, "ready")))
}

fn with_player<T>(
    state: &MpvEmbedState,
    callback: impl FnOnce(&mut MpvEmbedPlayer) -> Result<T, String>,
) -> Result<T, String> {
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    let player = player
        .as_mut()
        .ok_or_else(|| "mpv has no loaded media".to_string())?;

    callback(player)
}

impl MpvEmbedPlayer {
    fn snapshot(&mut self, hwnd: i64, fallback_status: &str) -> MpvEmbedSnapshot {
        let _ = self.host.resize();
        self.drain_events();
        let paused = self.mpv.get_property::<bool>("pause").unwrap_or(false);
        let ended = self.ended
            || self
                .mpv
                .get_property::<bool>("eof-reached")
                .unwrap_or(false);
        let position = self.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
        let duration = self.mpv.get_property::<f64>("duration").unwrap_or(0.0);
        let percent_pos = self.mpv.get_property::<f64>("percent-pos").unwrap_or(0.0);
        let near_end = duration.is_finite()
            && duration > 0.0
            && position.is_finite()
            && duration - position <= END_OF_MEDIA_SNAP_TOLERANCE_SECONDS
            && percent_pos.is_finite()
            && percent_pos >= 99.0;

        MpvEmbedSnapshot {
            path: self.path.clone(),
            hwnd,
            status: if ended {
                "ended"
            } else if paused {
                "paused"
            } else {
                fallback_status
            }
            .to_string(),
            ended,
            paused,
            position: if (ended || near_end) && duration.is_finite() && duration > 0.0 {
                duration
            } else {
                position
            },
            duration,
            volume: self.volume,
        }
    }

    fn drain_events(&mut self) {
        while let Some(event) = self.mpv.wait_event(0.0) {
            match event {
                Ok(Event::EndFile(mpv_end_file_reason::Eof)) => {
                    self.ended = true;
                }
                Ok(Event::StartFile | Event::Seek | Event::PlaybackRestart) => {
                    self.ended = false;
                }
                _ => {}
            }
        }
    }
}

impl MpvEmbedState {
    #[allow(dead_code)]
    pub fn resize_video_host(&self) -> Result<(), String> {
        let player = self
            .player
            .lock()
            .map_err(|_| "mpv embed state lock failed".to_string())?;

        if let Some(player) = player.as_ref() {
            player.host.resize()?;
        }

        Ok(())
    }
}

#[tauri::command]
pub fn mpv_embed_stop(state: State<'_, MpvEmbedState>) -> Result<(), String> {
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;

    if let Some(player) = player.take() {
        player
            .mpv
            .command("stop", &[])
            .map_err(|error| format!("mpv stop failed: {error}"))?;
    }

    Ok(())
}

fn create_embed_player(hwnd: i64) -> Result<libmpv2::Mpv, String> {
    libmpv2::Mpv::with_initializer(|initializer| {
        initializer.set_option("wid", hwnd)?;
        initializer.set_option("hwdec", "auto-safe")?;
        initializer.set_option("input-default-bindings", true)?;
        initializer.set_option("input-vo-keyboard", true)?;
        initializer.set_option("keep-open", true)?;
        initializer.set_option("load-scripts", true)?;
        initializer.set_option("osc", true)?;
        Ok(())
    })
    .map_err(|error| format!("mpv embed init failed: {error}"))
}

impl MpvVideoHost {
    fn new(parent_hwnd: i64) -> Result<Self, String> {
        let parent = parent_hwnd as isize as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        let class_name = wide_null("STATIC");
        let window_name = wide_null("OpenPlayer MPV Video Host");
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                layout.x,
                layout.y,
                layout.width,
                layout.height,
                parent,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };

        if hwnd.is_null() {
            return Err("failed to create native mpv child window".to_string());
        }

        unsafe {
            SetParent(hwnd, parent);
            MoveWindow(hwnd, layout.x, layout.y, layout.width, layout.height, 1);
            ShowWindow(hwnd, SW_SHOW);
        }

        Ok(Self {
            parent_hwnd: parent as isize,
            hwnd: hwnd as isize,
        })
    }

    fn resize(&self) -> Result<(), String> {
        let parent = self.parent_hwnd as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        unsafe {
            MoveWindow(
                self.hwnd as HWND,
                layout.x,
                layout.y,
                layout.width,
                layout.height,
                1,
            );
        }

        Ok(())
    }
}

impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd as HWND);
        }
    }
}

fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("enter a local media path for mpv embed playback".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_file() {
        return Err(format!("media path does not exist: {}", path.display()));
    }

    Ok(path)
}

fn window_hwnd(window: &impl HasWindowHandle) -> Result<i64, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as i64),
        _ => Err("mpv embed playback is only wired for Windows HWND targets".to_string()),
    }
}

fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn video_host_rect(parent_width: i32, parent_height: i32) -> VideoHostRect {
    let width = parent_width.max(1);
    let available_height = parent_height - VIDEO_HOST_TOP_RESERVE - VIDEO_HOST_BOTTOM_RESERVE;

    VideoHostRect {
        x: 0,
        y: VIDEO_HOST_TOP_RESERVE,
        width,
        height: available_height.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_media_path() {
        let error = validate_media_path("   ").expect_err("empty paths should be rejected");

        assert_eq!(error, "enter a local media path for mpv embed playback");
    }

    #[test]
    fn encodes_win32_class_name_with_null_terminator() {
        let encoded = wide_null("STATIC");

        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded[..6], [83, 84, 65, 84, 73, 67]);
    }

    #[test]
    fn reserves_web_controls_outside_native_video_host() {
        let rect = video_host_rect(1280, 720);

        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 1280);
        assert_eq!(rect.height, 720);
    }
}
