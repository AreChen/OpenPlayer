#[cfg(windows)]
use std::sync::Arc;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::{CStr, CString},
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use libmpv2::{events::Event, mpv_end_file_reason};
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde_json::Value;
#[cfg(windows)]
use tauri::WebviewWindow;
use tauri::{AppHandle, Manager, State, Window};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetClientRect, HWND_TOP, SW_HIDE, SW_SHOW, SWP_NOACTIVATE,
        SWP_SHOWWINDOW, SetParent, SetWindowPos, ShowWindow, WS_CHILD, WS_CLIPCHILDREN,
        WS_CLIPSIBLINGS, WS_VISIBLE,
    },
};

mod capture_recording;
pub(crate) mod commands;
mod constants;
#[cfg(target_os = "macos")]
mod macos_ffi;
mod media;
mod player;
mod player_events;
mod player_normalization;
mod player_resume;
mod player_snapshot;
mod player_tracks;
mod plugin_properties;
mod types;
mod video_host;
mod video_output;
mod wall;
mod wall_hosts;
mod wall_normalization;
mod wall_startup;
mod wall_state;
mod wall_status;

use capture_recording::*;
pub use commands::*;
use constants::*;
#[cfg(target_os = "macos")]
use macos_ffi::*;
use media::*;
use player::*;
use player_events::*;
use player_normalization::*;
#[cfg(test)]
use player_resume::*;
use player_tracks::*;
use plugin_properties::*;
pub(crate) use types::*;
#[cfg(test)]
use video_host::*;
use video_output::*;
use wall::*;
use wall_hosts::*;
use wall_normalization::*;
use wall_startup::*;
use wall_status::*;

#[cfg(test)]
mod tests;
