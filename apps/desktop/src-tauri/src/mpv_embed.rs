use std::{path::PathBuf, sync::Mutex};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Serialize;
use tauri::{State, Window};

#[derive(Default)]
pub struct MpvEmbedState {
    player: Mutex<Option<MpvEmbedPlayer>>,
}

struct MpvEmbedPlayer {
    mpv: libmpv2::Mpv,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvEmbedSnapshot {
    path: String,
    hwnd: i64,
    status: &'static str,
}

#[tauri::command]
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
    let hwnd = window_hwnd(window)?;
    let mpv = create_embed_player(hwnd)?;
    let path_text = path.to_string_lossy().to_string();

    mpv.command("loadfile", &[&path_text, "replace"])
        .map_err(|error| format!("mpv loadfile failed: {error}"))?;

    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    *player = Some(MpvEmbedPlayer { mpv });

    Ok(MpvEmbedSnapshot {
        path: path_text,
        hwnd,
        status: "playing",
    })
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
        initializer.set_option("keep-open", true)?;
        Ok(())
    })
    .map_err(|error| format!("mpv embed init failed: {error}"))
}

fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("enter a local media path for the mpv embed spike".to_string());
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
        _ => Err("mpv embed spike is only wired for Windows HWND targets".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_media_path() {
        let error = validate_media_path("   ").expect_err("empty paths should be rejected");

        assert_eq!(error, "enter a local media path for the mpv embed spike");
    }
}
