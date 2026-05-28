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

mod audio_export;
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
mod plugin_core;
mod plugin_properties;
#[cfg(any(windows, test))]
mod rtsp_telemetry;
#[cfg(any(windows, test))]
mod transport_latency;
mod types;
mod video_host;
mod video_output;
mod wall;
#[cfg(windows)]
mod wall_hosts;
#[cfg(any(windows, test))]
mod wall_low_latency;
#[cfg(any(windows, test))]
mod wall_normalization;
#[cfg(any(windows, test))]
mod wall_startup;
#[cfg(windows)]
mod wall_state;
#[cfg(any(windows, test))]
mod wall_status;

use audio_export::*;
use capture_recording::*;
pub use commands::*;
use constants::*;
#[cfg(target_os = "macos")]
use macos_ffi::*;
use media::*;
use player::*;
#[cfg(windows)]
use player_events::*;
use player_normalization::*;
#[cfg(test)]
use player_resume::*;
use player_snapshot::*;
use player_tracks::*;
use plugin_core::*;
use plugin_properties::*;
#[cfg(any(windows, test))]
use rtsp_telemetry::*;
#[cfg(any(windows, test))]
use transport_latency::*;
pub(crate) use types::*;
#[cfg(test)]
use video_host::*;
use video_output::*;
use wall::*;
#[cfg(windows)]
use wall_hosts::*;
#[cfg(any(windows, test))]
use wall_low_latency::*;
#[cfg(any(windows, test))]
use wall_normalization::*;
#[cfg(any(windows, test))]
use wall_startup::*;
#[cfg(any(windows, test))]
use wall_status::*;

#[cfg(test)]
mod tests;
