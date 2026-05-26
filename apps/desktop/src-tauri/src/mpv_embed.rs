use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::{CStr, CString},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(windows)]
use std::{collections::BTreeSet, sync::Arc};
#[cfg(target_os = "macos")]
use std::{
    ffi::{c_char, c_void},
    ptr,
};

use libmpv2::{events::Event, mpv_end_file_reason};
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::{Deserialize, Serialize};
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

#[cfg(windows)]
const VIDEO_HOST_TOP_RESERVE: i32 = 0;
#[cfg(windows)]
const VIDEO_HOST_BOTTOM_RESERVE: i32 = 0;
const END_OF_MEDIA_SNAP_TOLERANCE_SECONDS: f64 = 0.5;
const FRAME_STEP_SETTLE_INTERVAL: Duration = Duration::from_millis(6);
const FRAME_STEP_SETTLE_TIMEOUT: Duration = Duration::from_millis(180);
const FRAME_STEP_PAUSE_GUARD: Duration = Duration::from_millis(350);
const INITIAL_RESUME_SEEK_TIMEOUT: Duration = Duration::from_millis(8000);
const INITIAL_RESUME_SEEK_EVENT_WAIT: Duration = Duration::from_millis(80);
const INITIAL_RESUME_SEEK_SETTLE_TIMEOUT: Duration = Duration::from_millis(750);
const INITIAL_RESUME_SEEK_TOLERANCE_SECONDS: f64 = 1.0;
const RECORDING_OUTPUT_READY_TIMEOUT: Duration = Duration::from_secs(5);
const RECORDING_DUMP_PREROLL_SECONDS: f64 = 5.0;
const DEFAULT_VOLUME: f64 = 82.0;
const MIN_PLAYBACK_SPEED: f64 = 0.25;
const MAX_PLAYBACK_SPEED: f64 = 4.0;
const MIN_SUBTITLE_DELAY: f64 = -10.0;
const MAX_SUBTITLE_DELAY: f64 = 10.0;
const MAX_TRACKS: i64 = 128;
const SUPPORTED_SUBTITLE_EXTENSIONS: &[&str] = &["ass", "srt", "ssa", "sub", "vtt"];
const AUDIO_VISUALIZER_EXTENSIONS: &[&str] = &[
    "aac", "ac3", "adts", "aif", "aifc", "aiff", "alac", "amr", "ape", "au", "awb", "caf", "dff",
    "dsf", "dts", "dtshd", "eac3", "flac", "gsm", "m4a", "m4b", "m4r", "mka", "mlp", "mp1", "mp2",
    "mp3", "mpa", "mpc", "oga", "ogg", "opus", "ra", "snd", "spx", "tak", "tta", "voc", "wav",
    "weba", "wma", "wv",
];
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const OPENPLAYER_MPV_VO_ENV: &str = "OPENPLAYER_MPV_VO";
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const OPENPLAYER_MPV_GPU_CONTEXT_ENV: &str = "OPENPLAYER_MPV_GPU_CONTEXT";
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const OPENPLAYER_MPV_HWDEC_ENV: &str = "OPENPLAYER_MPV_HWDEC";
const MAX_MPV_WALL_TILES: usize = 16;
const MIN_MPV_WALL_TILE_RATIO: f64 = 0.02;
const MPV_WALL_TILE_START_STAGGER: Duration = Duration::from_millis(120);
const MPV_WALL_EVENT_DRAIN_LIMIT: usize = 32;
#[cfg(windows)]
const MPV_WALL_TILE_CORNER_RADIUS: i32 = 10;
#[cfg(windows)]
const MPV_WALL_TILE_BORDER_INSET: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Default)]
pub struct MpvWallState {
    #[cfg(windows)]
    players: Mutex<BTreeMap<String, MpvWallPlayer>>,
    #[cfg(windows)]
    starting: Mutex<BTreeSet<String>>,
    statuses: Mutex<BTreeMap<String, MpvWallTileSnapshot>>,
    generation: Mutex<u64>,
}

struct MpvEmbedPlayer {
    #[cfg(target_os = "macos")]
    _render_context: MacosMpvRenderContext,
    mpv: libmpv2::Mpv,
    host: MpvVideoHost,
    path: String,
    volume: f64,
    video_fill: bool,
    ended: bool,
    force_paused_until: Option<Instant>,
    recording: Option<MpvRecordingSession>,
}

#[cfg(windows)]
struct MpvWallPlayer {
    id: String,
    url: String,
    title: Option<String>,
    rect: MpvWallTileRect,
    mpv: Arc<libmpv2::Mpv>,
    host: MpvVideoHost,
}

#[cfg(windows)]
#[derive(Clone)]
struct MpvWallHostLayout {
    id: String,
    layout: VideoHostRect,
}

#[derive(Debug, Clone, PartialEq)]
struct MpvRecordingSession {
    path: String,
    format: String,
    method: MpvRecordingMethod,
}

#[derive(Debug, Clone, PartialEq)]
enum MpvRecordingMethod {
    StreamRecord,
    DumpCache { start_position: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitialResumeSeekReadiness {
    Ready,
    Wait,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MpvEventEffect {
    None,
    Active,
    Ended,
}

#[cfg(windows)]
struct MpvVideoHost {
    parent_hwnd: isize,
    hwnd: isize,
    corner_radius: i32,
}

#[cfg(target_os = "macos")]
struct MpvVideoHost {
    render_view: usize,
}

#[cfg(target_os = "macos")]
struct MacosMpvRenderContext {
    ctx: usize,
    view: usize,
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn openplayer_mpv_gl_view_create(parent: *mut c_void) -> *mut c_void;
    fn openplayer_mpv_gl_view_remove(view: *mut c_void);
    fn openplayer_mpv_gl_view_resize(view: *mut c_void);
    fn openplayer_mpv_gl_view_set_render_context(view: *mut c_void, render_context: *mut c_void);
    fn openplayer_mpv_gl_view_make_current(view: *mut c_void);
    fn openplayer_mpv_gl_view_draw(view: *mut c_void);
    fn openplayer_mpv_gl_get_proc_address(name: *const c_char) -> *mut c_void;
}

#[cfg(all(not(windows), not(target_os = "macos")))]
struct MpvVideoHost {
    wid: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MpvVideoOutputConfig {
    vo: Option<String>,
    gpu_context: Option<String>,
    hwdec: String,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct LinuxVideoOutputEnvironment<'a> {
    override_vo: Option<&'a str>,
    override_gpu_context: Option<&'a str>,
    override_hwdec: Option<&'a str>,
    has_dri_render_node: bool,
    virtual_drm_driver: bool,
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
    fps: f64,
    speed: f64,
    hwdec: String,
    video_fill: bool,
    subtitle_delay: f64,
    volume: f64,
    tracks: Vec<MpvEmbedTrack>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvEmbedTrack {
    id: i64,
    kind: String,
    title: Option<String>,
    language: Option<String>,
    codec: Option<String>,
    selected: bool,
    external: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MpvLoadOptions {
    #[serde(flatten)]
    options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileRequest {
    id: String,
    url: String,
    title: Option<String>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    muted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileLayout {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone)]
struct NormalizedMpvWallTileRequest {
    id: String,
    url: String,
    title: Option<String>,
    rect: MpvWallTileRect,
    muted: bool,
}

#[derive(Debug, Clone)]
struct NormalizedMpvWallTileLayout {
    id: String,
    rect: MpvWallTileRect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct MpvWallTileRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvWallTileSnapshot {
    id: String,
    url: String,
    title: Option<String>,
    status: String,
    latency_seconds: Option<f64>,
    buffer_seconds: Option<f64>,
    bitrate_bps: Option<f64>,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvCaptureArtifact {
    path: String,
    copied_to_clipboard: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvRecordingState {
    active: bool,
    path: Option<String>,
    format: Option<String>,
}

#[tauri::command]
#[allow(dead_code)]
pub fn mpv_embed_open_path(
    window: Window,
    state: State<'_, MpvEmbedState>,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    open_path_for_window(
        &window,
        state.inner(),
        path,
        resume_position,
        initial_volume,
        load_options,
    )
}

pub fn open_path_for_window(
    window: &impl HasWindowHandle,
    state: &MpvEmbedState,
    path: String,
    resume_position: Option<f64>,
    initial_volume: Option<f64>,
    load_options: Option<MpvLoadOptions>,
) -> Result<MpvEmbedSnapshot, String> {
    let path = validate_media_path(&path)?;
    let host = MpvVideoHost::new(window)?;
    let wid = host.wid();
    let mpv = create_embed_player(wid)?;
    #[cfg(target_os = "macos")]
    let render_context = create_macos_render_context(&mpv, &host)?;
    let path_text = path.to_string_lossy().to_string();
    let initial_volume = normalize_initial_volume(initial_volume)?;

    mpv.set_property("volume", initial_volume)
        .map_err(|error| format!("mpv initial volume failed: {error}"))?;
    configure_audio_visualizer(&mpv, &path);
    load_media_file(&mpv, &path_text, load_options.as_ref())?;
    load_sidecar_subtitles(&mpv, &path);

    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;
    if let Some(existing) = player.as_mut() {
        let _ = stop_recording_for_player(existing);
    }
    *player = Some(MpvEmbedPlayer {
        #[cfg(target_os = "macos")]
        _render_context: render_context,
        mpv,
        host,
        path: path_text,
        volume: initial_volume,
        video_fill: false,
        ended: false,
        force_paused_until: None,
        recording: None,
    });
    let next_player = player
        .as_mut()
        .ok_or_else(|| "mpv embed player initialization failed".to_string())?;
    next_player.apply_initial_resume_seek(resume_position);
    let snapshot = next_player.snapshot(wid, "playing");

    Ok(snapshot)
}

#[tauri::command]
pub async fn mpv_embed_play(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| {
        with_player(state, |player| {
            player.force_paused_until = None;
            player.ended = false;
            player
                .mpv
                .set_property("pause", false)
                .map_err(|error| format!("mpv play failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_pause(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| {
        with_player(state, |player| {
            player.force_paused_until = None;
            player
                .mpv
                .set_property("pause", true)
                .map_err(|error| format!("mpv pause failed: {error}"))?;
            Ok(player.snapshot(0, "paused"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_seek(app: AppHandle, position: f64) -> Result<MpvEmbedSnapshot, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("invalid mpv seek target".to_string());
    }

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player.force_paused_until = None;
            player.ended = false;
            player
                .mpv
                .command("seek", &[&position.to_string(), "absolute"])
                .map_err(|error| format!("mpv seek failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_frame_step(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| frame_step(state, "frame-step")).await
}

#[tauri::command]
pub async fn mpv_embed_frame_back_step(app: AppHandle) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, |state| frame_step(state, "frame-back-step")).await
}

#[tauri::command]
pub async fn mpv_embed_set_volume(app: AppHandle, volume: f64) -> Result<MpvEmbedSnapshot, String> {
    let volume = normalize_volume(volume)?;
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("volume", volume)
                .map_err(|error| format!("mpv volume failed: {error}"))?;
            player.volume = volume;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_speed(app: AppHandle, speed: f64) -> Result<MpvEmbedSnapshot, String> {
    let speed = normalize_playback_speed(speed)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("speed", speed)
                .map_err(|error| format!("mpv speed failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_hwdec(app: AppHandle, mode: String) -> Result<MpvEmbedSnapshot, String> {
    let hwdec = normalize_hwdec_mode(&mode)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("hwdec", hwdec)
                .map_err(|error| format!("mpv hardware decoding switch failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_video_fill(
    app: AppHandle,
    enabled: bool,
) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            set_video_fill_mode(&player.mpv, enabled)?;
            player.video_fill = enabled;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_loop_file(
    app: AppHandle,
    enabled: bool,
) -> Result<MpvEmbedSnapshot, String> {
    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("loop-file", if enabled { "inf" } else { "no" })
                .map_err(|error| format!("mpv loop-file mode failed: {error}"))?;
            if enabled {
                player.ended = false;
            }
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_subtitle_delay(
    app: AppHandle,
    delay: f64,
) -> Result<MpvEmbedSnapshot, String> {
    let delay = normalize_subtitle_delay(delay)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .set_property("sub-delay", delay)
                .map_err(|error| format!("mpv subtitle delay failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_set_plugin_property(
    app: AppHandle,
    property: String,
    value: Value,
) -> Result<MpvEmbedSnapshot, String> {
    let (property, value) = normalize_plugin_mpv_property(&property, &value)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            if plugin_subtitle_style_requires_ass_override(property) {
                player
                    .mpv
                    .set_property("sub-ass-override", "force")
                    .map_err(|error| format!("mpv subtitle style override failed: {error}"))?;
            }

            let targets = plugin_mpv_property_write_targets(property);
            let mut wrote_property = false;
            let mut first_error = None;
            for target in targets {
                match set_plugin_mpv_property_value(&player.mpv, target, &value) {
                    Ok(()) => wrote_property = true,
                    Err(error) => {
                        first_error.get_or_insert(error);
                    }
                }
            }
            if !wrote_property {
                let error = first_error.unwrap_or_else(|| "unknown error".to_string());
                return Err(format!("mpv plugin property failed: {error}"));
            }

            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_capture_screenshot(
    app: AppHandle,
    format: Option<String>,
    directory: Option<String>,
) -> Result<MpvCaptureArtifact, String> {
    let capture_directory = capture_directory_for_app(&app, directory)?;
    let format = normalize_capture_image_format(format)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            fs::create_dir_all(&capture_directory)
                .map_err(|error| format!("failed to create capture directory: {error}"))?;
            let output_path = capture_output_path(
                &capture_directory,
                &player.path,
                current_time_ms_for_capture(),
                &format,
            );
            let output_text = output_path.to_string_lossy().to_string();
            player
                .mpv
                .command("screenshot-to-file", &[&output_text, "video"])
                .map_err(|error| format!("mpv screenshot failed: {error}"))?;
            let copied_to_clipboard = copy_image_file_to_clipboard(&output_path).is_ok();
            Ok(MpvCaptureArtifact {
                path: output_text,
                copied_to_clipboard,
            })
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_recording_state(app: AppHandle) -> Result<MpvRecordingState, String> {
    let state = app.state::<MpvEmbedState>();
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;

    let Some(player) = player.as_mut() else {
        return Ok(MpvRecordingState::inactive(None));
    };
    player.drain_events();
    Ok(player.recording_state())
}

#[tauri::command]
pub async fn mpv_embed_start_recording(
    app: AppHandle,
    format: Option<String>,
    directory: Option<String>,
) -> Result<MpvRecordingState, String> {
    let recording_directory = recording_directory_for_app(&app, directory)?;
    let requested_format = normalize_recording_container_format(format)?;

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            if player.recording.is_some() {
                return Ok(player.recording_state());
            }

            fs::create_dir_all(&recording_directory)
                .map_err(|error| format!("failed to create recording directory: {error}"))?;
            let start_position = player.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
            let method = recording_method_for_media_path(&player.path, start_position);
            let format = recording_container_format_for_method(&method, &requested_format);
            let output_path = recording_output_path(
                &recording_directory,
                &player.path,
                current_time_ms_for_capture(),
                &format,
            );
            let output_text = output_path.to_string_lossy().to_string();
            match &method {
                MpvRecordingMethod::StreamRecord => {
                    player
                        .mpv
                        .set_property("stream-record", output_text.as_str())
                        .map_err(|error| format!("mpv recording start failed: {error}"))?;
                }
                MpvRecordingMethod::DumpCache { start_position } => {
                    let start_arg = recording_time_arg(*start_position)?;
                    player
                        .mpv
                        .command("async", &["dump-cache", &start_arg, "no", &output_text])
                        .map_err(|error| format!("mpv recording start failed: {error}"))?;
                }
            }
            player.recording = Some(MpvRecordingSession {
                path: output_text,
                format,
                method,
            });
            Ok(player.recording_state())
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_stop_recording(app: AppHandle) -> Result<MpvRecordingState, String> {
    run_mpv_command(app, move |state| {
        with_player(state, stop_recording_for_player)
    })
    .await
}

#[derive(Debug, PartialEq)]
enum PluginMpvPropertyValue {
    Text(String),
    Number(f64),
}

fn normalize_plugin_mpv_property(
    property: &str,
    value: &Value,
) -> Result<(&'static str, PluginMpvPropertyValue), String> {
    match property.trim() {
        "sub-font" => {
            let text = plugin_string_value(value)?;
            if text.trim().is_empty() || text.len() > 128 {
                return Err("invalid plugin subtitle font".to_string());
            }
            Ok(("sub-font", PluginMpvPropertyValue::Text(text)))
        }
        "sub-font-size" => {
            let size = plugin_number_value(value)?;
            if !(1.0..=128.0).contains(&size) {
                return Err("invalid plugin subtitle font size".to_string());
            }
            Ok(("sub-font-size", PluginMpvPropertyValue::Number(size)))
        }
        "sub-scale" => {
            let scale = plugin_number_value(value)?;
            if !(0.1..=5.0).contains(&scale) {
                return Err("invalid plugin subtitle scale".to_string());
            }
            Ok(("sub-scale", PluginMpvPropertyValue::Number(scale)))
        }
        "sub-pos" => {
            let position = plugin_number_value(value)?;
            if !(0.0..=100.0).contains(&position) {
                return Err("invalid plugin subtitle position".to_string());
            }
            Ok(("sub-pos", PluginMpvPropertyValue::Number(position)))
        }
        "sub-color" => {
            let color = plugin_string_value(value)?;
            if !is_plugin_hex_color(&color) {
                return Err("invalid plugin subtitle color".to_string());
            }
            Ok(("sub-color", PluginMpvPropertyValue::Text(color)))
        }
        "sub-spacing" => {
            let spacing = plugin_number_value(value)?;
            if !(-10.0..=10.0).contains(&spacing) {
                return Err("invalid plugin subtitle spacing".to_string());
            }
            Ok((
                "sub-spacing",
                PluginMpvPropertyValue::Text(format_plugin_number(spacing)),
            ))
        }
        "sub-outline-size" | "sub-border-size" => {
            let outline_size = plugin_number_value(value)?;
            if !(0.0..=32.0).contains(&outline_size) {
                return Err("invalid plugin subtitle outline size".to_string());
            }
            Ok((
                "sub-outline-size",
                PluginMpvPropertyValue::Number(outline_size),
            ))
        }
        "sub-shadow-offset" => {
            let shadow_offset = plugin_number_value(value)?;
            if !(0.0..=32.0).contains(&shadow_offset) {
                return Err("invalid plugin subtitle shadow offset".to_string());
            }
            Ok((
                "sub-shadow-offset",
                PluginMpvPropertyValue::Number(shadow_offset),
            ))
        }
        other => Err(format!("unsupported plugin mpv property: {other}")),
    }
}

fn plugin_string_value(value: &Value) -> Result<String, String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "plugin mpv property expects text".to_string())
}

fn plugin_number_value(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| "plugin mpv property expects a number".to_string())
}

fn format_plugin_number(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

fn set_plugin_mpv_property_value(
    mpv: &libmpv2::Mpv,
    property: &str,
    value: &PluginMpvPropertyValue,
) -> Result<(), String> {
    match value {
        PluginMpvPropertyValue::Text(value) => mpv
            .set_property(property, value.as_str())
            .map_err(|error| error.to_string()),
        PluginMpvPropertyValue::Number(value) => mpv
            .set_property(property, *value)
            .map_err(|error| error.to_string()),
    }
}

fn plugin_mpv_property_write_targets(property: &'static str) -> &'static [&'static str] {
    match property {
        "sub-font" => &["sub-font"],
        "sub-font-size" => &["sub-font-size"],
        "sub-scale" => &["sub-scale"],
        "sub-pos" => &["sub-pos"],
        "sub-color" => &["sub-color"],
        "sub-spacing" => &["sub-spacing"],
        "sub-outline-size" => &["sub-outline-size"],
        "sub-shadow-offset" => &["sub-shadow-offset"],
        _ => &[],
    }
}

fn plugin_subtitle_style_requires_ass_override(property: &str) -> bool {
    matches!(
        property,
        "sub-font"
            | "sub-font-size"
            | "sub-scale"
            | "sub-pos"
            | "sub-color"
            | "sub-spacing"
            | "sub-outline-size"
            | "sub-shadow-offset"
    )
}

fn is_plugin_hex_color(value: &str) -> bool {
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6) && hex.chars().all(|char| char.is_ascii_hexdigit())
}

#[tauri::command]
pub async fn mpv_embed_select_track(
    app: AppHandle,
    kind: String,
    track_id: Option<i64>,
) -> Result<MpvEmbedSnapshot, String> {
    let property = track_property_for_kind(&kind)?;
    if track_id.is_some_and(|id| id <= 0) {
        return Err("invalid mpv track id".to_string());
    }

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            if let Some(id) = track_id {
                player
                    .mpv
                    .set_property(property, id)
                    .map_err(|error| format!("mpv track selection failed: {error}"))?;
            } else {
                player
                    .mpv
                    .set_property(property, "no")
                    .map_err(|error| format!("mpv track disable failed: {error}"))?;
            }
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_add_subtitle(
    app: AppHandle,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    let path = validate_subtitle_path(&path)?;
    let path_text = path.to_string_lossy().to_string();

    run_mpv_command(app, move |state| {
        with_player(state, |player| {
            player
                .mpv
                .command("sub-add", &[&path_text, "select"])
                .map_err(|error| format!("mpv subtitle load failed: {error}"))?;
            Ok(player.snapshot(0, "playing"))
        })
    })
    .await
}

#[tauri::command]
pub async fn mpv_embed_snapshot(app: AppHandle) -> Result<Option<MpvEmbedSnapshot>, String> {
    run_mpv_command(app, |state| {
        let mut player = state
            .player
            .lock()
            .map_err(|_| "mpv embed state lock failed".to_string())?;

        Ok(player.as_mut().map(|player| player.snapshot(0, "playing")))
    })
    .await
}

#[tauri::command]
pub async fn mpv_wall_open(
    app: AppHandle,
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_open_for_app(&app, state.inner(), tiles)
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_layout(
    app: AppHandle,
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_layout_for_app(&app, state.inner(), tiles)
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_snapshot(app: AppHandle) -> Result<Vec<MpvWallTileSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_snapshot(state.inner())
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_close(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_close(&app, state.inner())
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
}

#[tauri::command]
pub async fn mpv_wall_set_visible(app: AppHandle, visible: bool) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvWallState>();
        wall_set_visible(&app, state.inner(), visible)
    })
    .await
    .map_err(|error| format!("mpv wall command task failed: {error}"))?
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

async fn run_mpv_command<T>(
    app: AppHandle,
    callback: impl FnOnce(&MpvEmbedState) -> Result<T, String> + Send + 'static,
) -> Result<T, String>
where
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<MpvEmbedState>();
        callback(state.inner())
    })
    .await
    .map_err(|error| format!("mpv command task failed: {error}"))?
}

#[cfg(windows)]
fn wall_open_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let normalized = normalize_wall_tile_requests(tiles)?;
    if state.can_reuse_open_wall(&normalized)? {
        let generation = state.current_generation()?;
        start_missing_wall_tiles(app, state, generation, &normalized)?;
        return wall_layout_for_app(
            app,
            state,
            normalized
                .iter()
                .map(|tile| MpvWallTileLayout {
                    id: tile.id.clone(),
                    x: tile.rect.x,
                    y: tile.rect.y,
                    width: tile.rect.width,
                    height: tile.rect.height,
                })
                .collect(),
        );
    }

    let generation = state.next_generation()?;
    let old_players = state.take_players()?;
    destroy_wall_players_on_main(app, old_players)?;
    let snapshots = wall_initial_snapshots(&normalized);
    state.replace_opening_state(snapshots.clone())?;

    start_missing_wall_tiles(app, state, generation, &normalized)?;

    Ok(snapshots)
}

#[cfg(not(windows))]
fn wall_open_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let _ = (app, state, tiles);
    Err("native multi-stream wall currently supports Windows".to_string())
}

#[cfg(windows)]
fn wall_layout_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let layouts = normalize_wall_tile_layouts(tiles)?;
    let host_window = wall_host_window(app)?;
    let mut host_layouts = Vec::new();
    let mut players = state
        .players
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;

    for layout in layouts {
        if let Some(player) = players.get_mut(&layout.id) {
            let host_layout = wall_tile_layout_for_window(&host_window, layout.rect)?;
            player.rect = layout.rect;
            host_layouts.push(MpvWallHostLayout {
                id: layout.id,
                layout: host_layout,
            });
        }
    }
    drop(players);

    schedule_wall_video_hosts_resize_on_main(app, host_layouts)?;
    wall_snapshot(state)
}

#[cfg(not(windows))]
fn wall_layout_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let _ = (app, state, tiles);
    Err("native multi-stream wall currently supports Windows".to_string())
}

#[cfg(windows)]
fn wall_snapshot(state: &MpvWallState) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let players = state
        .players
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    let mut statuses = state
        .statuses
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    for (id, player) in players.iter() {
        statuses.insert(id.clone(), player.live_snapshot());
    }
    Ok(statuses.values().cloned().collect())
}

#[cfg(not(windows))]
fn wall_snapshot(state: &MpvWallState) -> Result<Vec<MpvWallTileSnapshot>, String> {
    let _ = state;
    Ok(Vec::new())
}

#[cfg(windows)]
fn wall_close(app: &AppHandle, state: &MpvWallState) -> Result<(), String> {
    let _ = state.next_generation()?;
    let players = state.take_players()?;
    let mut starting = state
        .starting
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    starting.clear();
    drop(starting);
    let mut statuses = state
        .statuses
        .lock()
        .map_err(|_| "mpv wall state lock failed".to_string())?;
    statuses.clear();
    drop(statuses);
    destroy_wall_players_on_main(app, players)
}

#[cfg(windows)]
fn wall_set_visible(app: &AppHandle, state: &MpvWallState, visible: bool) -> Result<(), String> {
    let _ = state;
    let app_for_main = app.clone();
    app.run_on_main_thread(move || {
        let state = app_for_main.state::<MpvWallState>();
        if let Ok(players) = state.players.lock() {
            for player in players.values() {
                player.host.set_visible(visible);
            }
        }
    })
    .map_err(|error| format!("failed to schedule mpv wall visibility update: {error}"))
}

#[cfg(not(windows))]
fn wall_close(app: &AppHandle, state: &MpvWallState) -> Result<(), String> {
    let _ = (app, state);
    Ok(())
}

#[cfg(not(windows))]
fn wall_set_visible(app: &AppHandle, state: &MpvWallState, visible: bool) -> Result<(), String> {
    let _ = (app, state, visible);
    Ok(())
}

#[cfg(windows)]
impl MpvWallState {
    fn next_generation(&self) -> Result<u64, String> {
        let mut generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        *generation = generation.saturating_add(1);
        Ok(*generation)
    }

    fn current_generation(&self) -> Result<u64, String> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(*generation)
    }

    fn is_generation_current(&self, expected: u64) -> Result<bool, String> {
        let generation = self
            .generation
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(*generation == expected)
    }

    fn can_reuse_open_wall(&self, tiles: &[NormalizedMpvWallTileRequest]) -> Result<bool, String> {
        if tiles.is_empty() {
            return Ok(false);
        }
        let statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        if statuses.len() != tiles.len() {
            return Ok(false);
        }

        Ok(tiles.iter().all(|tile| {
            statuses
                .get(&tile.id)
                .is_some_and(|snapshot| snapshot.url == tile.url)
        }))
    }

    fn replace_opening_state(&self, snapshots: Vec<MpvWallTileSnapshot>) -> Result<(), String> {
        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        starting.clear();
        drop(starting);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        *statuses = snapshots
            .into_iter()
            .map(|snapshot| (snapshot.id.clone(), snapshot))
            .collect();
        Ok(())
    }

    fn take_players(&self) -> Result<BTreeMap<String, MpvWallPlayer>, String> {
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(std::mem::take(&mut *players))
    }

    fn insert_player(
        &self,
        generation: u64,
        player: MpvWallPlayer,
        status: &str,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let snapshot = player.status_snapshot(status, None);
        let id = player.id.clone();
        let _ = self.clear_tile_starting(&id);
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        players.insert(id.clone(), player);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(id, snapshot);
        Ok(())
    }

    fn mark_tile_starting(&self, generation: u64, id: &str) -> Result<bool, String> {
        if !self.is_generation_current(generation)? {
            return Ok(false);
        }
        let players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        if players.contains_key(id) {
            return Ok(false);
        }
        drop(players);

        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        Ok(starting.insert(id.to_string()))
    }

    fn clear_tile_starting(&self, id: &str) -> Result<(), String> {
        let mut starting = self
            .starting
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        starting.remove(id);
        Ok(())
    }

    fn update_player_status(
        &self,
        generation: u64,
        id: &str,
        status: &str,
        message: Option<String>,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        let Some(player) = players.get(id) else {
            return Ok(());
        };
        let snapshot = player.status_snapshot(status, message);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(id.to_string(), snapshot);
        Ok(())
    }

    fn update_tile_error(
        &self,
        generation: u64,
        tile: &NormalizedMpvWallTileRequest,
        message: String,
    ) -> Result<(), String> {
        if !self.is_generation_current(generation)? {
            return Ok(());
        }
        let _ = self.clear_tile_starting(&tile.id);
        let mut players = self
            .players
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        players.remove(&tile.id);
        drop(players);

        let mut statuses = self
            .statuses
            .lock()
            .map_err(|_| "mpv wall state lock failed".to_string())?;
        statuses.insert(
            tile.id.clone(),
            wall_tile_status_snapshot(tile, "error", Some(message)),
        );
        Ok(())
    }
}

#[cfg(windows)]
fn start_missing_wall_tiles(
    app: &AppHandle,
    state: &MpvWallState,
    generation: u64,
    tiles: &[NormalizedMpvWallTileRequest],
) -> Result<(), String> {
    for (index, tile) in tiles.iter().cloned().enumerate() {
        if state.mark_tile_starting(generation, &tile.id)? {
            spawn_wall_tile_start(
                app.clone(),
                generation,
                wall_request_id(generation, index),
                tile,
                MPV_WALL_TILE_START_STAGGER.saturating_mul(index as u32),
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn spawn_wall_tile_start(
    app: AppHandle,
    generation: u64,
    request_id: u64,
    tile: NormalizedMpvWallTileRequest,
    delay: Duration,
) {
    let _ = thread::Builder::new()
        .name(format!("openplayer-wall-{}", tile.id))
        .spawn(move || {
            if !delay.is_zero() {
                thread::sleep(delay);
            }
            let state = app.state::<MpvWallState>();
            if let Err(error) =
                wall_start_tile_for_app(&app, state.inner(), generation, request_id, &tile)
            {
                let _ = state
                    .inner()
                    .update_tile_error(generation, &tile, error.to_string());
            }
        });
}

#[cfg(windows)]
fn wall_start_tile_for_app(
    app: &AppHandle,
    state: &MpvWallState,
    generation: u64,
    request_id: u64,
    tile: &NormalizedMpvWallTileRequest,
) -> Result<(), String> {
    if !state.is_generation_current(generation)? {
        return Ok(());
    }

    let host = create_wall_video_host_on_main(app, tile.rect)?;
    let mpv = Arc::new(create_embed_player_without_logs(host.wid())?);
    configure_wall_osd(mpv.as_ref());
    if tile.muted {
        mpv.set_property("volume", 0.0)
            .map_err(|error| format!("mpv wall mute failed: {error}"))?;
    }
    state.insert_player(
        generation,
        MpvWallPlayer {
            id: tile.id.clone(),
            url: tile.url.clone(),
            title: tile.title.clone(),
            rect: tile.rect,
            mpv: Arc::clone(&mpv),
            host,
        },
        "loading",
    )?;

    if !state.is_generation_current(generation)? {
        return Ok(());
    }

    load_media_file_async(mpv.as_ref(), &tile.url, None, request_id)?;
    state.update_player_status(generation, &tile.id, "playing", None)
}

fn wall_request_id(generation: u64, index: usize) -> u64 {
    generation
        .saturating_mul(1_000)
        .saturating_add(index as u64)
        .saturating_add(1)
}

#[cfg(windows)]
fn create_wall_video_host_on_main(
    app: &AppHandle,
    rect: MpvWallTileRect,
) -> Result<MpvVideoHost, String> {
    let host_window = wall_host_window(app)?;
    let layout = wall_tile_layout_for_window(&host_window, rect)?;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let result =
            MpvVideoHost::new_with_layout(&host_window, layout, MPV_WALL_TILE_CORNER_RADIUS);
        let _ = sender.send(result);
    })
    .map_err(|error| format!("failed to schedule mpv wall host creation: {error}"))?;

    receiver
        .recv()
        .map_err(|_| "mpv wall host creation did not return a result".to_string())?
}

#[cfg(windows)]
fn destroy_wall_players_on_main(
    app: &AppHandle,
    players: BTreeMap<String, MpvWallPlayer>,
) -> Result<(), String> {
    if players.is_empty() {
        return Ok(());
    }

    let mut hosts = Vec::with_capacity(players.len());
    for (_, player) in players {
        let MpvWallPlayer { mpv, host, .. } = player;
        let _ = mpv.command("stop", &[]);
        hosts.push(host);
    }

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        for host in &mut hosts {
            host.destroy();
        }
        let _ = sender.send(());
    })
    .map_err(|error| format!("failed to schedule mpv wall host teardown: {error}"))?;

    receiver
        .recv()
        .map_err(|_| "mpv wall host teardown did not return a result".to_string())
}

#[cfg(windows)]
fn schedule_wall_video_hosts_resize_on_main(
    app: &AppHandle,
    layouts: Vec<MpvWallHostLayout>,
) -> Result<(), String> {
    if layouts.is_empty() {
        return Ok(());
    }

    let app_for_main = app.clone();
    app.run_on_main_thread(move || {
        let state = app_for_main.state::<MpvWallState>();
        if let Ok(mut players) = state.players.lock() {
            for host_layout in layouts {
                if let Some(player) = players.get_mut(&host_layout.id) {
                    let _ = player.host.resize_to_layout(host_layout.layout);
                }
            }
        }
    })
    .map_err(|error| format!("failed to schedule mpv wall host resize: {error}"))
}

#[cfg(windows)]
fn wall_host_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("overlay")
        .or_else(|| app.get_webview_window("main"))
        .ok_or_else(|| "mpv wall host window is unavailable".to_string())
}

fn wall_initial_snapshots(tiles: &[NormalizedMpvWallTileRequest]) -> Vec<MpvWallTileSnapshot> {
    tiles
        .iter()
        .map(|tile| wall_tile_status_snapshot(tile, "loading", None))
        .collect()
}

fn wall_tile_status_snapshot(
    tile: &NormalizedMpvWallTileRequest,
    status: &str,
    message: Option<String>,
) -> MpvWallTileSnapshot {
    MpvWallTileSnapshot {
        id: tile.id.clone(),
        url: tile.url.clone(),
        title: tile.title.clone(),
        status: status.to_string(),
        latency_seconds: None,
        buffer_seconds: None,
        bitrate_bps: None,
        message,
    }
}

#[cfg(windows)]
impl MpvWallPlayer {
    fn live_snapshot(&self) -> MpvWallTileSnapshot {
        wall_player_snapshot(self)
    }

    fn status_snapshot(&self, status: &str, message: Option<String>) -> MpvWallTileSnapshot {
        MpvWallTileSnapshot {
            id: self.id.clone(),
            url: self.url.clone(),
            title: self.title.clone(),
            status: status.to_string(),
            latency_seconds: None,
            buffer_seconds: None,
            bitrate_bps: None,
            message,
        }
    }
}

#[cfg(any(windows, test))]
fn wall_live_status(eof_reached: bool, paused: bool, idle: bool) -> &'static str {
    if eof_reached {
        "ended"
    } else if paused {
        "paused"
    } else if idle {
        "loading"
    } else {
        "playing"
    }
}

#[cfg(any(windows, test))]
fn combine_wall_bitrate(
    video_bitrate: Option<f64>,
    audio_bitrate: Option<f64>,
    raw_input_bytes_per_second: Option<f64>,
) -> Option<f64> {
    let track_bitrate = video_bitrate
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(0.0)
        + audio_bitrate
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(0.0);
    if track_bitrate > 0.0 {
        return Some(track_bitrate);
    }

    raw_input_bytes_per_second
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|bytes_per_second| bytes_per_second * 8.0)
}

#[cfg(windows)]
fn read_finite_mpv_property(mpv: &libmpv2::Mpv, property: &str) -> Option<f64> {
    mpv.get_property::<f64>(property)
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .or_else(|| {
            mpv.get_property::<i64>(property)
                .ok()
                .map(|value| value as f64)
                .filter(|value| value.is_finite() && *value >= 0.0)
        })
}

#[cfg(windows)]
fn read_wall_buffer(mpv: &libmpv2::Mpv) -> Option<f64> {
    read_finite_mpv_property(mpv, "demuxer-cache-duration")
        .or_else(|| read_finite_mpv_property(mpv, "demuxer-cache-state/cache-duration"))
        .or_else(|| read_finite_mpv_property(mpv, "cache-duration"))
        .or_else(|| {
            let cache_time = read_finite_mpv_property(mpv, "demuxer-cache-time")?;
            let position = read_finite_mpv_property(mpv, "time-pos")?;
            let buffered = cache_time - position;
            (buffered.is_finite() && buffered >= 0.0).then_some(buffered)
        })
}

#[cfg(windows)]
fn read_wall_bitrate(mpv: &libmpv2::Mpv) -> Option<f64> {
    combine_wall_bitrate(
        read_finite_mpv_property(mpv, "video-bitrate"),
        read_finite_mpv_property(mpv, "audio-bitrate"),
        read_finite_mpv_property(mpv, "cache-speed")
            .or_else(|| read_finite_mpv_property(mpv, "demuxer-cache-state/raw-input-rate")),
    )
}

#[cfg(windows)]
fn configure_wall_osd(mpv: &libmpv2::Mpv) {
    let _ = mpv.set_property("osd-align-x", "left");
    let _ = mpv.set_property("osd-align-y", "top");
    let _ = mpv.set_property("osd-margin-x", 12);
    let _ = mpv.set_property("osd-margin-y", 12);
    let _ = mpv.set_property("osd-font-size", 18);
    let _ = mpv.set_property("osd-bold", true);
    let _ = mpv.set_property("osd-color", "#f1c66b");
    let _ = mpv.set_property("osd-border-color", "#120f08");
    let _ = mpv.set_property("osd-border-size", 1.8);
    let _ = mpv.set_property("osd-shadow-color", "#000000");
    let _ = mpv.set_property("osd-shadow-offset", 1.0);
    let _ = mpv.set_property("osd-back-color", "#99000000");
}

#[cfg(any(windows, test))]
fn format_wall_buffer_millis(buffer_seconds: Option<f64>) -> String {
    buffer_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|value| format!("{} ms", (value * 1000.0).round() as i64))
        .unwrap_or_else(|| "-- ms".to_string())
}

#[cfg(any(windows, test))]
fn format_wall_bitrate(bits_per_second: Option<f64>) -> String {
    let Some(bits_per_second) = bits_per_second.filter(|value| value.is_finite() && *value > 0.0)
    else {
        return "--".to_string();
    };
    if bits_per_second >= 1_000_000.0 {
        format!("{:.1} Mbps", bits_per_second / 1_000_000.0)
    } else {
        format!("{} Kbps", (bits_per_second / 1000.0).round() as i64)
    }
}

#[cfg(windows)]
fn update_wall_osd(mpv: &libmpv2::Mpv, buffer_seconds: Option<f64>, bitrate_bps: Option<f64>) {
    let text = format!(
        "BUF {} · {}",
        format_wall_buffer_millis(buffer_seconds),
        format_wall_bitrate(bitrate_bps)
    );
    let _ = mpv.command("show-text", &[text.as_str(), "1500", "1"]);
}

#[cfg(windows)]
fn drain_wall_player_events(mpv: &libmpv2::Mpv) {
    for _ in 0..MPV_WALL_EVENT_DRAIN_LIMIT {
        let Some(event) = mpv.wait_event(0.0) else {
            break;
        };
        let _ = handle_mpv_event(event);
    }
}

#[cfg(windows)]
fn read_wall_bool_property(mpv: &libmpv2::Mpv, property: &str) -> bool {
    mpv.get_property::<bool>(property).unwrap_or(false)
}

#[cfg(windows)]
fn wall_player_snapshot(player: &MpvWallPlayer) -> MpvWallTileSnapshot {
    drain_wall_player_events(player.mpv.as_ref());
    let status = wall_live_status(
        read_wall_bool_property(player.mpv.as_ref(), "eof-reached"),
        read_wall_bool_property(player.mpv.as_ref(), "pause"),
        read_wall_bool_property(player.mpv.as_ref(), "idle-active"),
    );
    let buffer_seconds = read_wall_buffer(player.mpv.as_ref());
    let bitrate_bps = read_wall_bitrate(player.mpv.as_ref());
    update_wall_osd(player.mpv.as_ref(), buffer_seconds, bitrate_bps);

    MpvWallTileSnapshot {
        id: player.id.clone(),
        url: player.url.clone(),
        title: player.title.clone(),
        status: status.to_string(),
        latency_seconds: None,
        buffer_seconds,
        bitrate_bps,
        message: None,
    }
}

fn frame_step(state: &MpvEmbedState, command: &str) -> Result<MpvEmbedSnapshot, String> {
    with_player(state, |player| {
        player
            .mpv
            .command(command, &[])
            .map_err(|error| format!("mpv {command} failed: {error}"))?;
        player.force_paused_until = Some(Instant::now() + FRAME_STEP_PAUSE_GUARD);
        settle_frame_step_pause(&player.mpv)?;
        Ok(player.snapshot(0, "paused"))
    })
}

fn settle_frame_step_pause(mpv: &libmpv2::Mpv) -> Result<(), String> {
    thread::sleep(FRAME_STEP_SETTLE_INTERVAL);
    let deadline = Instant::now() + FRAME_STEP_SETTLE_TIMEOUT;
    while Instant::now() < deadline {
        if mpv.get_property::<bool>("pause").unwrap_or(false) {
            return Ok(());
        }
        thread::sleep(FRAME_STEP_SETTLE_INTERVAL);
    }

    mpv.set_property("pause", true)
        .map_err(|error| format!("mpv frame-step pause settle failed: {error}"))
}

fn valid_fps(value: f64) -> Option<f64> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

fn read_player_fps(mpv: &libmpv2::Mpv) -> f64 {
    mpv.get_property::<f64>("container-fps")
        .ok()
        .and_then(valid_fps)
        .or_else(|| {
            mpv.get_property::<f64>("estimated-vf-fps")
                .ok()
                .and_then(valid_fps)
        })
        .unwrap_or(0.0)
}

fn read_optional_string(mpv: &libmpv2::Mpv, property: &str) -> Option<String> {
    mpv.get_property::<String>(property)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn read_tracks(mpv: &libmpv2::Mpv) -> Vec<MpvEmbedTrack> {
    let count = mpv
        .get_property::<i64>("track-list/count")
        .unwrap_or(0)
        .clamp(0, MAX_TRACKS);
    let mut tracks = Vec::new();

    for index in 0..count {
        let id = match mpv.get_property::<i64>(&format!("track-list/{index}/id")) {
            Ok(value) if value > 0 => value,
            _ => continue,
        };
        let kind = match mpv.get_property::<String>(&format!("track-list/{index}/type")) {
            Ok(value) if matches!(value.as_str(), "audio" | "video" | "sub") => value,
            _ => continue,
        };

        tracks.push(MpvEmbedTrack {
            id,
            kind,
            title: read_optional_string(mpv, &format!("track-list/{index}/title")),
            language: read_optional_string(mpv, &format!("track-list/{index}/lang")),
            codec: read_optional_string(mpv, &format!("track-list/{index}/codec")),
            selected: mpv
                .get_property::<bool>(&format!("track-list/{index}/selected"))
                .unwrap_or(false),
            external: mpv
                .get_property::<bool>(&format!("track-list/{index}/external"))
                .unwrap_or(false),
        });
    }

    tracks
}

fn normalize_playback_speed(speed: f64) -> Result<f64, String> {
    if !speed.is_finite() {
        return Err("invalid mpv playback speed".to_string());
    }

    Ok(speed.clamp(MIN_PLAYBACK_SPEED, MAX_PLAYBACK_SPEED))
}

fn normalize_subtitle_delay(delay: f64) -> Result<f64, String> {
    if !delay.is_finite() {
        return Err("invalid mpv subtitle delay".to_string());
    }

    Ok(delay.clamp(MIN_SUBTITLE_DELAY, MAX_SUBTITLE_DELAY))
}

fn set_video_fill_mode(mpv: &libmpv2::Mpv, enabled: bool) -> Result<(), String> {
    let panscan = if enabled { 1.0 } else { 0.0 };
    mpv.set_property("panscan", panscan)
        .map_err(|error| format!("mpv video layout failed: {error}"))
}

fn normalize_hwdec_mode(mode: &str) -> Result<&'static str, String> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "hardware" | "auto" | "auto-safe" => Ok("auto-safe"),
        "software" | "no" | "off" => Ok("no"),
        _ => Err("invalid mpv hardware decoding mode".to_string()),
    }
}

fn track_property_for_kind(kind: &str) -> Result<&'static str, String> {
    match kind {
        "audio" => Ok("aid"),
        "video" => Ok("vid"),
        "subtitle" | "sub" => Ok("sid"),
        _ => Err("invalid mpv track kind".to_string()),
    }
}

fn normalize_initial_resume_position(position: Option<f64>) -> Option<f64> {
    position.filter(|position| position.is_finite() && *position > 0.0)
}

fn normalize_volume(volume: f64) -> Result<f64, String> {
    if !volume.is_finite() {
        return Err("invalid mpv volume".to_string());
    }

    Ok(volume.clamp(0.0, 100.0))
}

fn normalize_initial_volume(volume: Option<f64>) -> Result<f64, String> {
    volume.map_or(Ok(DEFAULT_VOLUME), normalize_volume)
}

fn initial_resume_seek_readiness(
    target_position: f64,
    duration: f64,
    seekable: bool,
) -> InitialResumeSeekReadiness {
    if !target_position.is_finite() || target_position <= 0.0 {
        return InitialResumeSeekReadiness::Skip;
    }

    if seekable || (duration.is_finite() && duration > 0.0 && target_position < duration) {
        return InitialResumeSeekReadiness::Ready;
    }

    if !duration.is_finite() || duration <= 0.0 || target_position >= duration {
        return InitialResumeSeekReadiness::Wait;
    }

    InitialResumeSeekReadiness::Wait
}

fn is_transient_initial_resume_seek_error(error: &libmpv2::Error) -> bool {
    matches!(error, libmpv2::Error::Raw(code) if *code == libmpv2::mpv_error::Command)
}

fn handle_mpv_event(event: libmpv2::Result<Event<'_>>) -> MpvEventEffect {
    match event {
        Ok(Event::EndFile(mpv_end_file_reason::Eof)) => MpvEventEffect::Ended,
        Ok(Event::StartFile | Event::FileLoaded | Event::Seek | Event::PlaybackRestart) => {
            MpvEventEffect::Active
        }
        Ok(Event::LogMessage {
            prefix,
            level,
            text,
            ..
        }) => {
            log_mpv_video_diagnostic(prefix, level, text);
            MpvEventEffect::None
        }
        Err(error) => {
            eprintln!("OpenPlayer mpv event failed: {error}");
            MpvEventEffect::None
        }
        _ => MpvEventEffect::None,
    }
}

impl MpvEmbedPlayer {
    fn apply_initial_resume_seek(&mut self, resume_position: Option<f64>) {
        let Some(target_position) = normalize_initial_resume_position(resume_position) else {
            return;
        };

        let deadline = Instant::now() + INITIAL_RESUME_SEEK_TIMEOUT;

        loop {
            if !self.wait_for_initial_resume_seek(target_position, deadline) {
                return;
            }

            match self
                .mpv
                .command("seek", &[&target_position.to_string(), "absolute"])
            {
                Ok(()) => {
                    self.ended = false;
                    self.settle_initial_resume_seek(target_position);
                    return;
                }
                Err(error) if is_transient_initial_resume_seek_error(&error) => {
                    if Instant::now() >= deadline {
                        eprintln!("OpenPlayer initial resume seek timed out: {error}");
                        return;
                    }
                    self.wait_for_mpv_event(deadline, INITIAL_RESUME_SEEK_EVENT_WAIT);
                }
                Err(error) => {
                    eprintln!("OpenPlayer initial resume seek skipped: {error}");
                    return;
                }
            }
        }
    }

    fn wait_for_initial_resume_seek(&mut self, target_position: f64, deadline: Instant) -> bool {
        loop {
            let duration = self.mpv.get_property::<f64>("duration").unwrap_or(0.0);
            let seekable = self.mpv.get_property::<bool>("seekable").unwrap_or(false);
            match initial_resume_seek_readiness(target_position, duration, seekable) {
                InitialResumeSeekReadiness::Ready => return true,
                InitialResumeSeekReadiness::Skip => return false,
                InitialResumeSeekReadiness::Wait => {}
            }

            let now = Instant::now();
            if now >= deadline {
                return false;
            }

            self.wait_for_mpv_event(deadline, INITIAL_RESUME_SEEK_EVENT_WAIT);
        }
    }

    fn settle_initial_resume_seek(&mut self, target_position: f64) {
        let deadline = Instant::now() + INITIAL_RESUME_SEEK_SETTLE_TIMEOUT;

        loop {
            let position = self.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
            if position.is_finite()
                && (position - target_position).abs() <= INITIAL_RESUME_SEEK_TOLERANCE_SECONDS
            {
                return;
            }

            let now = Instant::now();
            if now >= deadline {
                return;
            }

            self.wait_for_mpv_event(deadline, INITIAL_RESUME_SEEK_EVENT_WAIT);
        }
    }

    fn wait_for_mpv_event(&mut self, deadline: Instant, max_wait: Duration) {
        let now = Instant::now();
        if now >= deadline {
            return;
        }

        let wait = deadline
            .saturating_duration_since(now)
            .min(max_wait)
            .as_secs_f64();
        if let Some(event) = self.mpv.wait_event(wait) {
            let effect = handle_mpv_event(event);
            self.apply_mpv_event_effect(effect);
        }
    }

    fn apply_mpv_event_effect(&mut self, effect: MpvEventEffect) {
        match effect {
            MpvEventEffect::Active => {
                self.ended = false;
            }
            MpvEventEffect::Ended => {
                self.ended = true;
                let _ = stop_recording_for_player(self);
            }
            MpvEventEffect::None => {}
        }
    }

    fn recording_state(&self) -> MpvRecordingState {
        if let Some(recording) = &self.recording {
            MpvRecordingState {
                active: true,
                path: Some(recording.path.clone()),
                format: Some(recording.format.clone()),
            }
        } else {
            MpvRecordingState::inactive(None)
        }
    }

    fn snapshot(&mut self, hwnd: i64, fallback_status: &str) -> MpvEmbedSnapshot {
        self.drain_events();
        let raw_paused = self.mpv.get_property::<bool>("pause").unwrap_or(false);
        let pause_guard_active = self
            .force_paused_until
            .is_some_and(|deadline| Instant::now() < deadline);
        if !pause_guard_active {
            self.force_paused_until = None;
        }
        let paused = raw_paused || pause_guard_active;
        let ended = self.ended
            || self
                .mpv
                .get_property::<bool>("eof-reached")
                .unwrap_or(false);
        let position = self.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
        let duration = self.mpv.get_property::<f64>("duration").unwrap_or(0.0);
        let fps = read_player_fps(&self.mpv);
        let speed = self.mpv.get_property::<f64>("speed").unwrap_or(1.0);
        let hwdec = self
            .mpv
            .get_property::<String>("hwdec")
            .unwrap_or_else(|_| "auto-safe".to_string());
        let subtitle_delay = self.mpv.get_property::<f64>("sub-delay").unwrap_or(0.0);
        let tracks = read_tracks(&self.mpv);
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
            fps,
            speed,
            hwdec,
            video_fill: self.video_fill,
            subtitle_delay: if subtitle_delay.is_finite() {
                subtitle_delay
            } else {
                0.0
            },
            volume: self.volume,
            tracks,
        }
    }

    fn drain_events(&mut self) {
        while let Some(event) = self.mpv.wait_event(0.0) {
            let effect = handle_mpv_event(event);
            self.apply_mpv_event_effect(effect);
        }
    }
}

impl MpvRecordingState {
    fn inactive(path: Option<String>) -> Self {
        Self {
            active: false,
            path,
            format: None,
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
pub fn mpv_embed_stop(window: Window, state: State<'_, MpvEmbedState>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if MainThreadMarker::new().is_none() {
            let app = window.app_handle().clone();
            let app_for_stop = app.clone();
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            app.run_on_main_thread(move || {
                let state = app_for_stop.state::<MpvEmbedState>();
                let _ = sender.send(stop_player(state.inner()));
            })
            .map_err(|error| {
                format!("failed to schedule macOS mpv AppKit host teardown: {error}")
            })?;

            return receiver.recv().map_err(|_| {
                "macOS mpv AppKit host teardown did not return a result".to_string()
            })?;
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = window;

    stop_player(state.inner())
}

fn stop_player(state: &MpvEmbedState) -> Result<(), String> {
    let mut player = state
        .player
        .lock()
        .map_err(|_| "mpv embed state lock failed".to_string())?;

    if let Some(mut player) = player.take() {
        let _ = stop_recording_for_player(&mut player);
        player
            .mpv
            .command("stop", &[])
            .map_err(|error| format!("mpv stop failed: {error}"))?;
    }

    Ok(())
}

fn create_embed_player(hwnd: i64) -> Result<libmpv2::Mpv, String> {
    create_embed_player_with_log_subscription(hwnd, true)
}

fn create_embed_player_without_logs(hwnd: i64) -> Result<libmpv2::Mpv, String> {
    create_embed_player_with_log_subscription(hwnd, false)
}

fn create_embed_player_with_log_subscription(
    hwnd: i64,
    subscribe_logs: bool,
) -> Result<libmpv2::Mpv, String> {
    prepare_libmpv_numeric_locale()?;
    let video_output_config = platform_video_output_config();
    log_selected_mpv_video_output_config(&video_output_config);

    let mpv = libmpv2::Mpv::with_initializer(|initializer| {
        #[cfg(not(target_os = "macos"))]
        initializer.set_option("wid", hwnd)?;
        #[cfg(target_os = "macos")]
        let _ = hwnd;
        configure_native_video_output(&initializer, &video_output_config)?;
        #[cfg(target_os = "macos")]
        initializer.set_option("video-timing-offset", "0")?;
        initializer.set_option("input-default-bindings", false)?;
        initializer.set_option("input-vo-keyboard", false)?;
        initializer.set_option("keep-open", true)?;
        initializer.set_option("load-scripts", true)?;
        initializer.set_option("osc", false)?;
        Ok(())
    })
    .map_err(|error| format!("mpv embed init failed: {error}"))?;

    if subscribe_logs {
        request_mpv_log_messages(&mpv);
    }

    Ok(mpv)
}

#[cfg(target_os = "macos")]
fn create_macos_render_context(
    mpv: &libmpv2::Mpv,
    host: &MpvVideoHost,
) -> Result<MacosMpvRenderContext, String> {
    unsafe {
        openplayer_mpv_gl_view_make_current(host.render_view_ptr());
    }

    let mut init_params = libmpv2_sys::mpv_opengl_init_params {
        get_proc_address: Some(macos_mpv_get_proc_address),
        get_proc_address_ctx: ptr::null_mut(),
    };
    let mut render_params = [
        libmpv2_sys::mpv_render_param {
            type_: libmpv2_sys::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            data: libmpv2_sys::MPV_RENDER_API_TYPE_OPENGL.as_ptr() as *mut c_void,
        },
        libmpv2_sys::mpv_render_param {
            type_: libmpv2_sys::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
            data: (&mut init_params as *mut libmpv2_sys::mpv_opengl_init_params).cast(),
        },
        libmpv2_sys::mpv_render_param {
            type_: 0,
            data: ptr::null_mut(),
        },
    ];
    let mut context: *mut libmpv2_sys::mpv_render_context = ptr::null_mut();
    let result = unsafe {
        libmpv2_sys::mpv_render_context_create(
            &mut context,
            mpv.ctx.as_ptr(),
            render_params.as_mut_ptr(),
        )
    };
    if result < 0 {
        return Err(format!(
            "mpv render context init failed: {}",
            mpv_error_message(result)
        ));
    }

    unsafe {
        openplayer_mpv_gl_view_set_render_context(host.render_view_ptr(), context.cast());
        libmpv2_sys::mpv_render_context_set_update_callback(
            context,
            Some(macos_mpv_render_update),
            host.render_view_ptr(),
        );
    }

    Ok(MacosMpvRenderContext {
        ctx: context as usize,
        view: host.render_view,
    })
}

#[cfg(target_os = "macos")]
impl Drop for MacosMpvRenderContext {
    fn drop(&mut self) {
        let context = self.ctx as *mut libmpv2_sys::mpv_render_context;
        unsafe {
            libmpv2_sys::mpv_render_context_set_update_callback(context, None, ptr::null_mut());
            openplayer_mpv_gl_view_set_render_context(self.view as *mut c_void, ptr::null_mut());
            libmpv2_sys::mpv_render_context_free(context);
        }
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn macos_mpv_get_proc_address(
    _ctx: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    unsafe { openplayer_mpv_gl_get_proc_address(name) }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn macos_mpv_render_update(ctx: *mut c_void) {
    unsafe {
        openplayer_mpv_gl_view_draw(ctx);
    }
}

fn mpv_error_message(code: i32) -> String {
    let message = unsafe { libmpv2_sys::mpv_error_string(code) };
    if message.is_null() {
        return code.to_string();
    }

    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}

fn configure_native_video_output(
    initializer: &libmpv2::MpvInitializer,
    config: &MpvVideoOutputConfig,
) -> libmpv2::Result<()> {
    apply_video_output_config(initializer, config)
}

fn apply_video_output_config(
    initializer: &libmpv2::MpvInitializer,
    config: &MpvVideoOutputConfig,
) -> libmpv2::Result<()> {
    if let Some(vo) = config.vo.as_ref() {
        initializer.set_option("vo", vo.as_str())?;
    }
    if let Some(gpu_context) = config.gpu_context.as_ref() {
        initializer.set_option("gpu-context", gpu_context.as_str())?;
    }
    initializer.set_option("hwdec", config.hwdec.as_str())?;
    Ok(())
}

fn request_mpv_log_messages(mpv: &libmpv2::Mpv) {
    let Ok(min_level) = CString::new("v") else {
        return;
    };
    let result =
        unsafe { libmpv2_sys::mpv_request_log_messages(mpv.ctx.as_ptr(), min_level.as_ptr()) };
    if result < 0 {
        eprintln!("OpenPlayer mpv log subscription failed: {result}");
    }
}

fn log_selected_mpv_video_output_config(config: &MpvVideoOutputConfig) {
    eprintln!(
        "OpenPlayer mpv video output: vo={}, gpu-context={}, hwdec={}",
        config.vo.as_deref().unwrap_or("mpv-default"),
        config.gpu_context.as_deref().unwrap_or("mpv-default"),
        config.hwdec
    );
}

fn log_mpv_video_diagnostic(prefix: &str, level: &str, text: &str) {
    if is_mpv_video_diagnostic_log(level, prefix, text) {
        eprintln!(
            "OpenPlayer mpv {level}/{prefix}: {}",
            text.trim_end_matches(['\r', '\n'])
        );
    }
}

fn is_mpv_video_diagnostic_log(level: &str, prefix: &str, text: &str) -> bool {
    let level = level.to_ascii_lowercase();
    if matches!(level.as_str(), "fatal" | "error" | "warn") {
        return true;
    }

    let prefix = prefix.to_ascii_lowercase();
    if prefix.starts_with("vo") || matches!(prefix.as_str(), "vd" | "ffmpeg/video") {
        return true;
    }

    let text = text.to_ascii_lowercase();
    text.contains("vo:")
        || text.contains("[vo")
        || text.contains("gpu")
        || text.contains("egl")
        || text.contains("dri")
        || text.contains("vaapi")
        || text.contains("vdpau")
        || text.contains("hwdec")
}

#[cfg(target_os = "linux")]
fn platform_video_output_config() -> MpvVideoOutputConfig {
    let override_vo = std::env::var(OPENPLAYER_MPV_VO_ENV).ok();
    let override_gpu_context = std::env::var(OPENPLAYER_MPV_GPU_CONTEXT_ENV).ok();
    let override_hwdec = std::env::var(OPENPLAYER_MPV_HWDEC_ENV).ok();

    resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
        override_vo: override_vo.as_deref(),
        override_gpu_context: override_gpu_context.as_deref(),
        override_hwdec: override_hwdec.as_deref(),
        has_dri_render_node: has_linux_dri_render_node(),
        virtual_drm_driver: has_virtual_linux_drm_driver(),
    })
}

#[cfg(target_os = "macos")]
fn platform_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("libmpv".to_string()),
        gpu_context: None,
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg(all(not(target_os = "linux"), not(target_os = "macos")))]
fn platform_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: None,
        gpu_context: None,
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn resolve_linux_video_output_config(
    environment: LinuxVideoOutputEnvironment<'_>,
) -> MpvVideoOutputConfig {
    let override_vo = normalized_override(environment.override_vo);
    let override_gpu_context = normalized_override(environment.override_gpu_context);
    let override_hwdec = normalized_override(environment.override_hwdec);

    if let Some(vo) = override_vo {
        let vo_lower = vo.to_ascii_lowercase();
        let mut config = if vo_lower == "x11" {
            x11_software_video_output_config()
        } else {
            x11_gpu_video_output_config()
        };
        config.vo = Some(vo);
        if vo_lower != "gpu" && override_gpu_context.is_none() {
            config.gpu_context = None;
        }
        if let Some(gpu_context) = override_gpu_context {
            config.gpu_context = Some(gpu_context);
        }
        if let Some(hwdec) = override_hwdec {
            config.hwdec = hwdec;
        }
        return config;
    }

    let mut config = if environment.has_dri_render_node && !environment.virtual_drm_driver {
        x11_gpu_video_output_config()
    } else {
        x11_software_video_output_config()
    };

    if config.vo.as_deref() == Some("gpu")
        && let Some(gpu_context) = override_gpu_context
    {
        config.gpu_context = Some(gpu_context);
    }
    if let Some(hwdec) = override_hwdec {
        config.hwdec = hwdec;
    }

    config
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn x11_software_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("x11".to_string()),
        gpu_context: None,
        hwdec: "no".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn x11_gpu_video_output_config() -> MpvVideoOutputConfig {
    MpvVideoOutputConfig {
        vo: Some("gpu".to_string()),
        gpu_context: Some("x11egl".to_string()),
        hwdec: "auto-safe".to_string(),
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn normalized_override(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn has_linux_dri_render_node() -> bool {
    let Ok(entries) = fs::read_dir("/dev/dri") else {
        return false;
    };

    entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with("renderD"))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn has_virtual_linux_drm_driver() -> bool {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return false;
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_link(entry.path().join("device/driver")).ok())
        .filter_map(|driver| {
            driver
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToOwned::to_owned)
        })
        .any(|driver| is_virtual_linux_drm_driver(&driver))
}

fn is_virtual_linux_drm_driver(driver: &str) -> bool {
    let driver = driver.to_ascii_lowercase().replace('_', "-");

    matches!(
        driver.as_str(),
        "bochs" | "bochs-drm" | "cirrus" | "qxl" | "virtio-gpu"
    )
}

#[cfg(unix)]
fn prepare_libmpv_numeric_locale() -> Result<(), String> {
    let locale = std::ffi::CString::new("C")
        .map_err(|_| "failed to prepare LC_NUMERIC=C for libmpv".to_string())?;
    // SAFETY: libmpv requires the process C numeric locale to be "C" before
    // mpv_create(). We set only LC_NUMERIC immediately before initializing mpv.
    let result = unsafe { libc::setlocale(libc::LC_NUMERIC, locale.as_ptr()) };
    if result.is_null() {
        Err("failed to set LC_NUMERIC=C before libmpv initialization".to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(unix))]
fn prepare_libmpv_numeric_locale() -> Result<(), String> {
    Ok(())
}

impl MpvVideoHost {
    #[cfg(windows)]
    fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        let parent_hwnd = window_hwnd(window)?;
        let parent = parent_hwnd as isize as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        Self::new_with_layout(window, layout, 0)
    }

    #[cfg(windows)]
    fn new_with_layout(
        window: &impl HasWindowHandle,
        layout: VideoHostRect,
        corner_radius: i32,
    ) -> Result<Self, String> {
        let parent_hwnd = window_hwnd(window)?;
        let parent = parent_hwnd as isize as HWND;
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
        }
        if let Err(error) = position_video_host(hwnd, layout)
            .and_then(|()| apply_video_host_region(hwnd, layout, corner_radius))
        {
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err(error);
        }
        unsafe {
            ShowWindow(hwnd, SW_SHOW);
        }

        Ok(Self {
            parent_hwnd: parent as isize,
            hwnd: hwnd as isize,
            corner_radius,
        })
    }

    #[cfg(target_os = "macos")]
    fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        let parent_ns_view = window_appkit_ns_view(window)?;
        let Some(_mtm) = MainThreadMarker::new() else {
            return Err("macOS mpv video host must be created on the main thread".to_string());
        };

        let render_view =
            unsafe { openplayer_mpv_gl_view_create(parent_ns_view as *mut c_void) } as usize;
        if render_view == 0 {
            return Err("failed to create macOS mpv OpenGL render view".to_string());
        }

        Ok(Self { render_view })
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        Ok(Self {
            wid: window_mpv_wid(window)?,
        })
    }

    #[cfg(windows)]
    fn wid(&self) -> i64 {
        self.hwnd as i64
    }

    #[cfg(target_os = "macos")]
    fn wid(&self) -> i64 {
        self.render_view as i64
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    fn wid(&self) -> i64 {
        self.wid
    }

    #[cfg(windows)]
    fn resize(&self) -> Result<(), String> {
        let parent = self.parent_hwnd as HWND;
        let mut rect = RECT::default();
        if unsafe { GetClientRect(parent, &mut rect) } == 0 {
            return Err("failed to read Tauri client size for mpv child window".to_string());
        }

        let layout = video_host_rect(rect.right - rect.left, rect.bottom - rect.top);
        position_video_host(self.hwnd as HWND, layout)
    }

    #[cfg(windows)]
    fn resize_to_layout(&self, layout: VideoHostRect) -> Result<(), String> {
        position_video_host(self.hwnd as HWND, layout)
            .and_then(|()| apply_video_host_region(self.hwnd as HWND, layout, self.corner_radius))
    }

    #[cfg(windows)]
    fn set_visible(&self, visible: bool) {
        if self.hwnd == 0 {
            return;
        }
        unsafe {
            ShowWindow(self.hwnd as HWND, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    #[cfg(windows)]
    fn destroy(&mut self) {
        if self.hwnd == 0 {
            return;
        }
        unsafe {
            DestroyWindow(self.hwnd as HWND);
        }
        self.hwnd = 0;
        self.parent_hwnd = 0;
    }

    #[cfg(target_os = "macos")]
    fn resize(&self) -> Result<(), String> {
        unsafe {
            openplayer_mpv_gl_view_resize(self.render_view_ptr());
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn render_view_ptr(&self) -> *mut c_void {
        self.render_view as *mut c_void
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    fn resize(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(windows)]
fn position_video_host(hwnd: HWND, layout: VideoHostRect) -> Result<(), String> {
    let result = unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOP,
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    };
    if result == 0 {
        Err("failed to position mpv child window above the video surface".to_string())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn apply_video_host_region(
    hwnd: HWND,
    layout: VideoHostRect,
    corner_radius: i32,
) -> Result<(), String> {
    if corner_radius <= 0 {
        return Ok(());
    }

    let diameter = corner_radius.saturating_mul(2).max(1);
    let region = unsafe {
        CreateRoundRectRgn(
            0,
            0,
            layout.width.max(1).saturating_add(1),
            layout.height.max(1).saturating_add(1),
            diameter,
            diameter,
        )
    };
    if region.is_null() {
        return Err("failed to create rounded mpv child window region".to_string());
    }

    if unsafe { SetWindowRgn(hwnd, region, 1) } == 0 {
        unsafe {
            DeleteObject(region);
        }
        Err("failed to apply rounded mpv child window region".to_string())
    } else {
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(target_os = "macos")]
impl Drop for MpvVideoHost {
    fn drop(&mut self) {
        unsafe {
            openplayer_mpv_gl_view_remove(self.render_view_ptr());
        }
    }
}

fn normalize_wall_tile_requests(
    tiles: Vec<MpvWallTileRequest>,
) -> Result<Vec<NormalizedMpvWallTileRequest>, String> {
    if tiles.is_empty() {
        return Ok(Vec::new());
    }
    if tiles.len() > MAX_MPV_WALL_TILES {
        return Err(format!(
            "mpv wall supports at most {MAX_MPV_WALL_TILES} streams"
        ));
    }

    let mut ids = BTreeMap::new();
    let mut normalized = Vec::with_capacity(tiles.len());
    for tile in tiles {
        let tile = normalize_wall_tile_request(tile)?;
        if ids.insert(tile.id.clone(), ()).is_some() {
            return Err(format!("duplicate mpv wall tile id: {}", tile.id));
        }
        normalized.push(tile);
    }
    Ok(normalized)
}

fn normalize_wall_tile_request(
    tile: MpvWallTileRequest,
) -> Result<NormalizedMpvWallTileRequest, String> {
    let id = normalize_wall_tile_id(&tile.id)?;
    let url = validate_media_path(&tile.url)?
        .to_string_lossy()
        .to_string();
    let title = tile
        .title
        .map(|title| title.trim().chars().take(128).collect::<String>())
        .filter(|title| !title.is_empty());
    Ok(NormalizedMpvWallTileRequest {
        id,
        url,
        title,
        rect: normalize_wall_tile_rect(tile.x, tile.y, tile.width, tile.height)?,
        muted: tile.muted.unwrap_or(true),
    })
}

fn normalize_wall_tile_layouts(
    tiles: Vec<MpvWallTileLayout>,
) -> Result<Vec<NormalizedMpvWallTileLayout>, String> {
    if tiles.len() > MAX_MPV_WALL_TILES {
        return Err(format!(
            "mpv wall supports at most {MAX_MPV_WALL_TILES} layout items"
        ));
    }
    tiles
        .into_iter()
        .map(|tile| {
            Ok(NormalizedMpvWallTileLayout {
                id: normalize_wall_tile_id(&tile.id)?,
                rect: normalize_wall_tile_rect(tile.x, tile.y, tile.width, tile.height)?,
            })
        })
        .collect()
}

fn normalize_wall_tile_id(id: &str) -> Result<String, String> {
    let id = id.trim();
    if id.is_empty()
        || id.len() > 64
        || !id
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '_' | '-'))
    {
        return Err(format!("mpv wall tile id is invalid: {id}"));
    }
    Ok(id.to_string())
}

fn normalize_wall_tile_rect(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<MpvWallTileRect, String> {
    if ![x, y, width, height].into_iter().all(f64::is_finite) {
        return Err("mpv wall tile layout must use finite numbers".to_string());
    }
    let x = x.clamp(0.0, 1.0 - MIN_MPV_WALL_TILE_RATIO);
    let y = y.clamp(0.0, 1.0 - MIN_MPV_WALL_TILE_RATIO);
    let width = width.clamp(MIN_MPV_WALL_TILE_RATIO, 1.0 - x);
    let height = height.clamp(MIN_MPV_WALL_TILE_RATIO, 1.0 - y);
    Ok(MpvWallTileRect {
        x,
        y,
        width,
        height,
    })
}

fn wall_tile_rect_to_video_host_rect(
    parent_width: i32,
    parent_height: i32,
    rect: MpvWallTileRect,
) -> VideoHostRect {
    let parent_width = parent_width.max(1);
    let parent_height = parent_height.max(1);
    let x = (rect.x * f64::from(parent_width)).round() as i32;
    let y = (rect.y * f64::from(parent_height)).round() as i32;
    let max_width = parent_width.saturating_sub(x).max(1);
    let max_height = parent_height.saturating_sub(y).max(1);
    let width = ((rect.width * f64::from(parent_width)).round() as i32)
        .max(1)
        .min(max_width);
    let height = ((rect.height * f64::from(parent_height)).round() as i32)
        .max(1)
        .min(max_height);
    VideoHostRect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(windows)]
fn wall_tile_layout_for_window(
    window: &impl HasWindowHandle,
    rect: MpvWallTileRect,
) -> Result<VideoHostRect, String> {
    let parent_hwnd = window_hwnd(window)?;
    let parent = parent_hwnd as isize as HWND;
    let mut client = RECT::default();
    if unsafe { GetClientRect(parent, &mut client) } == 0 {
        return Err("failed to read window size for mpv wall tile".to_string());
    }
    Ok(inset_wall_video_host_rect(
        wall_tile_rect_to_video_host_rect(
            client.right - client.left,
            client.bottom - client.top,
            rect,
        ),
    ))
}

#[cfg(windows)]
fn inset_wall_video_host_rect(layout: VideoHostRect) -> VideoHostRect {
    let inset = MPV_WALL_TILE_BORDER_INSET
        .min(layout.width / 3)
        .min(layout.height / 3);
    if inset <= 0 {
        return layout;
    }
    VideoHostRect {
        x: layout.x + inset,
        y: layout.y + inset,
        width: (layout.width - inset.saturating_mul(2)).max(1),
        height: (layout.height - inset.saturating_mul(2)).max(1),
    }
}

fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("enter a local media path for mpv embed playback".to_string());
    }

    if trimmed.contains("://") {
        validate_media_stream_url(trimmed)?;
        return Ok(PathBuf::from(trimmed));
    }

    let path = PathBuf::from(trimmed);
    if !path.is_file() {
        return Err(format!("media path does not exist: {}", path.display()));
    }

    Ok(path)
}

fn validate_media_stream_url(url: &str) -> Result<(), String> {
    if url.len() > 2048 || url.chars().any(char::is_whitespace) {
        return Err("media stream url is invalid".to_string());
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return Err("media stream url must include a protocol".to_string());
    };
    if rest.trim_matches('/').is_empty() {
        return Err("media stream url must include a host or path".to_string());
    }
    if is_supported_media_stream_scheme(&scheme.to_ascii_lowercase()) {
        Ok(())
    } else {
        Err(format!("unsupported media stream protocol: {scheme}"))
    }
}

fn is_supported_media_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme,
        "http" | "https" | "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

fn load_media_file(
    mpv: &libmpv2::Mpv,
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<(), String> {
    let args = loadfile_args_for_media_path(path_text, load_options)?;
    let arg_refs = loadfile_arg_refs(&args);
    match mpv.command("loadfile", &arg_refs) {
        Ok(()) => Ok(()),
        Err(error) if is_hls_manifest_media_url(path_text) => {
            let legacy_args = legacy_hls_loadfile_args_for_media_path(path_text, load_options)?;
            let legacy_arg_refs = loadfile_arg_refs(&legacy_args);
            mpv.command("loadfile", &legacy_arg_refs)
                .map_err(|legacy_error| {
                    format!(
                        "mpv loadfile failed: {error}; legacy HLS loadfile failed: {legacy_error}"
                    )
                })
        }
        Err(error) => Err(format!("mpv loadfile failed: {error}")),
    }
}

fn load_media_file_async(
    mpv: &libmpv2::Mpv,
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
    request_id: u64,
) -> Result<(), String> {
    let args = loadfile_args_for_media_path(path_text, load_options)?;
    let arg_refs = loadfile_arg_refs(&args);
    mpv_command_async(mpv, request_id, "loadfile", &arg_refs)
}

fn mpv_command_async(
    mpv: &libmpv2::Mpv,
    request_id: u64,
    name: &str,
    args: &[&str],
) -> Result<(), String> {
    let mut cstr_args = Vec::with_capacity(args.len() + 1);
    cstr_args
        .push(CString::new(name).map_err(|error| format!("mpv command name failed: {error}"))?);

    for arg in args {
        cstr_args.push(
            CString::new(*arg).map_err(|error| format!("mpv command argument failed: {error}"))?,
        );
    }

    let mut ptrs: Vec<_> = cstr_args.iter().map(|cstr| cstr.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    let result =
        unsafe { libmpv2_sys::mpv_command_async(mpv.ctx.as_ptr(), request_id, ptrs.as_mut_ptr()) };
    if result < 0 {
        Err(format!(
            "mpv {name} async failed: {}",
            mpv_error_message(result)
        ))
    } else {
        Ok(())
    }
}

fn loadfile_arg_refs(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

fn loadfile_args_for_media_path(
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<Vec<String>, String> {
    let options = loadfile_options_for_media_path(path_text, load_options)?;
    let mut args = vec![path_text.to_string(), "replace".to_string()];
    if let Some(options) = options {
        args.push("-1".to_string());
        args.push(options);
    }
    Ok(args)
}

fn legacy_hls_loadfile_args_for_media_path(
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<Vec<String>, String> {
    let options = loadfile_options_for_media_path(path_text, load_options)?;
    let mut args = vec![path_text.to_string(), "replace".to_string()];
    if let Some(options) = options {
        args.push(options);
    }
    Ok(args)
}

fn loadfile_options_for_media_path(
    path_text: &str,
    load_options: Option<&MpvLoadOptions>,
) -> Result<Option<String>, String> {
    let mut options = BTreeMap::new();
    if is_hls_manifest_media_url(path_text) {
        options.insert("demuxer".to_string(), "+lavf".to_string());
        options.insert("demuxer-lavf-format".to_string(), "hls".to_string());
    }

    if let Some(load_options) = load_options {
        for (key, value) in normalize_mpv_load_options(load_options)? {
            options.insert(key, value);
        }
    }

    if options.is_empty() {
        Ok(None)
    } else {
        Ok(Some(
            options
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(","),
        ))
    }
}

fn normalize_mpv_load_options(
    load_options: &MpvLoadOptions,
) -> Result<Vec<(String, String)>, String> {
    let mut normalized = Vec::new();
    for (key, value) in &load_options.options {
        let key = key.trim().to_ascii_lowercase();
        if !is_supported_mpv_load_option_key(&key) {
            return Err(format!("unsupported mpv load option: {key}"));
        }
        if !is_valid_mpv_load_option_value(value) {
            return Err(format!("invalid mpv load option value for {key}"));
        }
        normalized.push((key, value.trim().to_string()));
    }
    Ok(normalized)
}

fn is_supported_mpv_load_option_key(key: &str) -> bool {
    matches!(key, "demuxer" | "demuxer-lavf-format")
}

fn is_valid_mpv_load_option_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && !value.contains(',')
        && !value.contains('=')
        && !value.chars().any(char::is_control)
}

fn is_hls_manifest_media_url(path_text: &str) -> bool {
    let Some((scheme, rest)) = path_text.trim().split_once("://") else {
        return false;
    };
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return false;
    }
    let path_without_fragment = rest.split_once('#').map(|(path, _)| path).unwrap_or(rest);
    let path_without_query = path_without_fragment
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(path_without_fragment);
    path_without_query.to_ascii_lowercase().ends_with(".m3u8")
}

fn capture_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create capture directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().picture_dir() {
        directory.push("OpenPlayer");
        directory.push("Captures");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve capture directory: {error}"))?;
    directory.push("captures");
    Ok(directory)
}

fn recording_directory_for_app(
    app: &AppHandle,
    directory_override: Option<String>,
) -> Result<PathBuf, String> {
    if let Some(directory) = normalize_capture_directory_override(directory_override)? {
        fs::create_dir_all(&directory)
            .map_err(|error| format!("failed to create recording directory: {error}"))?;
        return Ok(directory);
    }

    if let Ok(mut directory) = app.path().video_dir() {
        directory.push("OpenPlayer");
        directory.push("Recordings");
        return Ok(directory);
    }

    let mut directory = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve recording directory: {error}"))?;
    directory.push("recordings");
    Ok(directory)
}

fn capture_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> PathBuf {
    let stem = capture_file_stem(media_path);
    directory.join(format!("openplayer-{stem}-{timestamp_ms}.{format}"))
}

fn recording_output_path(
    directory: &Path,
    media_path: &str,
    timestamp_ms: u64,
    format: &str,
) -> PathBuf {
    let stem = capture_file_stem(media_path);
    directory.join(format!("openplayer-{stem}-{timestamp_ms}.{format}"))
}

fn normalize_capture_image_format(format: Option<String>) -> Result<String, String> {
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("png")
        .to_ascii_lowercase();
    match format.as_str() {
        "png" | "jpg" | "webp" => Ok(format),
        "jpeg" => Ok("jpg".to_string()),
        _ => Err(format!("unsupported screenshot format: {format}")),
    }
}

fn normalize_recording_container_format(format: Option<String>) -> Result<String, String> {
    let format = format
        .as_deref()
        .map(str::trim)
        .filter(|format| !format.is_empty())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    match format.as_str() {
        "mp4" | "mkv" | "ts" => Ok(format),
        _ => Err(format!("unsupported recording format: {format}")),
    }
}

fn recording_container_format_for_method(
    method: &MpvRecordingMethod,
    requested_format: &str,
) -> String {
    match method {
        MpvRecordingMethod::DumpCache { .. } | MpvRecordingMethod::StreamRecord => {
            requested_format.to_string()
        }
    }
}

fn normalize_capture_directory_override(
    directory: Option<String>,
) -> Result<Option<PathBuf>, String> {
    let Some(directory) = directory
        .as_deref()
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
    else {
        return Ok(None);
    };
    if directory.len() > 1024 {
        return Err("capture directory path is too long".to_string());
    }
    let path = PathBuf::from(directory);
    if !path.is_absolute() {
        return Err("capture directory path must be absolute".to_string());
    }
    if path.is_file() {
        return Err("capture directory path is not a directory".to_string());
    }
    Ok(Some(path))
}

fn stop_recording_for_player(player: &mut MpvEmbedPlayer) -> Result<MpvRecordingState, String> {
    let Some(recording) = player.recording.take() else {
        return Ok(MpvRecordingState::inactive(None));
    };
    match recording.method {
        MpvRecordingMethod::StreamRecord => {
            player
                .mpv
                .set_property("stream-record", "")
                .map_err(|error| format!("mpv recording stop failed: {error}"))?;
        }
        MpvRecordingMethod::DumpCache { .. } => {
            let _ = player.mpv.command("dump-cache", &["0", "0", ""]);
        }
    }
    wait_for_recording_output(&recording.path, RECORDING_OUTPUT_READY_TIMEOUT)?;
    Ok(MpvRecordingState::inactive(Some(recording.path)))
}

fn recording_method_for_media_path(media_path: &str, start_position: f64) -> MpvRecordingMethod {
    if media_stream_scheme(media_path).is_some_and(is_live_recording_stream_scheme) {
        MpvRecordingMethod::StreamRecord
    } else {
        MpvRecordingMethod::DumpCache {
            start_position: recording_dump_start_position(start_position),
        }
    }
}

fn media_stream_scheme(media_path: &str) -> Option<&str> {
    media_path
        .split_once("://")
        .map(|(scheme, _)| scheme)
        .filter(|scheme| !scheme.is_empty())
}

fn is_live_recording_stream_scheme(scheme: &str) -> bool {
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "rtmp" | "rtmps" | "rtsp" | "rtsps" | "srt" | "udp"
    )
}

fn recording_dump_start_position(position: f64) -> f64 {
    if !position.is_finite() {
        return 0.0;
    }
    (position - RECORDING_DUMP_PREROLL_SECONDS).max(0.0)
}

fn recording_time_arg(position: f64) -> Result<String, String> {
    if !position.is_finite() {
        return Err("recording start time is invalid".to_string());
    }
    Ok(format!("{:.3}", position.max(0.0)))
}

fn wait_for_recording_output(path: &str, timeout: Duration) -> Result<(), String> {
    let path = Path::new(path);
    let deadline = Instant::now() + timeout;
    loop {
        if recording_output_has_content(path).unwrap_or(false) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return ensure_recording_output_has_content(path);
        }
        thread::sleep(Duration::from_millis(40));
    }
}

fn recording_output_has_content(path: &Path) -> Result<bool, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("recording output was not created: {error}"))?;
    Ok(metadata.len() > 0)
}

fn ensure_recording_output_has_content(path: &Path) -> Result<(), String> {
    if !recording_output_has_content(path)? {
        let _ = fs::remove_file(path);
        return Err(
            "mpv produced an empty recording file; try recording for longer or using MKV"
                .to_string(),
        );
    }
    Ok(())
}

fn copy_image_file_to_clipboard(path: &Path) -> Result<(), String> {
    let image = image::ImageReader::open(path)
        .map_err(|error| format!("failed to open screenshot for clipboard: {error}"))?
        .decode()
        .map_err(|error| format!("failed to decode screenshot for clipboard: {error}"))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|error| format!("failed to access clipboard: {error}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(image.into_raw()),
        })
        .map_err(|error| format!("failed to copy screenshot to clipboard: {error}"))
}

fn capture_file_stem(media_path: &str) -> String {
    let normalized = media_path.replace('\\', "/");
    let tail = normalized
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or("capture");
    let stem = tail
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(tail)
        .trim();
    let mut sanitized = String::new();
    for char in stem.chars() {
        if char.is_ascii_alphanumeric() || matches!(char, '-' | '_') {
            sanitized.push(char);
        } else if char.is_whitespace() || matches!(char, '.' | ':' | '/' | '\\') {
            sanitized.push('_');
        }
        if sanitized.len() >= 80 {
            break;
        }
    }
    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "capture".to_string()
    } else {
        sanitized
    }
}

fn current_time_ms_for_capture() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn validate_subtitle_path(path: &str) -> Result<PathBuf, String> {
    let path = validate_media_path(path)?;
    if is_supported_subtitle_path(&path) {
        Ok(path)
    } else {
        Err(format!("unsupported subtitle file: {}", path.display()))
    }
}

fn is_supported_subtitle_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            SUPPORTED_SUBTITLE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
        .unwrap_or(false)
}

fn configure_audio_visualizer(mpv: &libmpv2::Mpv, path: &Path) {
    if !is_likely_audio_path(path) {
        return;
    }

    if let Err(error) = mpv.set_property("audio-display", "no") {
        eprintln!("OpenPlayer mpv audio visualizer: failed to disable cover art: {error}");
    }
}

fn is_likely_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            AUDIO_VISUALIZER_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        })
        .unwrap_or(false)
}

fn discover_sidecar_subtitles(media_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = media_path.parent() else {
        return Vec::new();
    };
    let Some(media_stem) = media_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    let mut subtitles: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| is_supported_subtitle_path(path))
        .filter(|path| is_matching_sidecar_stem(path, media_stem))
        .collect();

    subtitles.sort_by(|left, right| {
        sidecar_sort_key(left, media_stem).cmp(&sidecar_sort_key(right, media_stem))
    });
    subtitles
}

fn is_matching_sidecar_stem(path: &Path, media_stem: &str) -> bool {
    let Some(candidate_stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    if candidate_stem == media_stem {
        return true;
    }

    candidate_stem
        .strip_prefix(media_stem)
        .and_then(|suffix| suffix.chars().next())
        .is_some_and(|separator| matches!(separator, '.' | '-' | '_'))
}

fn sidecar_sort_key(path: &Path, media_stem: &str) -> (u8, String) {
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let exact_rank = if file_stem == media_stem { 0 } else { 1 };
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    (exact_rank, file_name)
}

fn load_sidecar_subtitles(mpv: &libmpv2::Mpv, media_path: &Path) {
    for (index, subtitle) in discover_sidecar_subtitles(media_path).iter().enumerate() {
        let subtitle_text = subtitle.to_string_lossy();
        let mode = if index == 0 { "select" } else { "auto" };
        let _ = mpv.command("sub-add", &[subtitle_text.as_ref(), mode]);
    }
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
fn window_mpv_wid(window: &impl HasWindowHandle) -> Result<i64, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    mpv_wid_from_raw_window_handle(handle.as_raw())
}

#[cfg(target_os = "macos")]
fn window_appkit_ns_view(window: &impl HasWindowHandle) -> Result<usize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    match handle.as_raw() {
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as usize),
        _ => Err("window operation is only wired for macOS AppKit NSView targets".to_string()),
    }
}

#[cfg_attr(target_os = "macos", allow(dead_code))]
fn mpv_wid_from_raw_window_handle(handle: RawWindowHandle) -> Result<i64, String> {
    match handle {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as i64),
        RawWindowHandle::Xlib(handle) if handle.window > 0 => xlib_window_to_mpv_wid(handle.window),
        RawWindowHandle::Xcb(handle) => Ok(i64::from(handle.window.get())),
        RawWindowHandle::Wayland(_) => Err(
            "mpv embed playback currently supports Windows HWND and X11 window hosts; Wayland video host support is not implemented yet"
                .to_string(),
        ),
        RawWindowHandle::AppKit(handle) => Ok(handle.ns_view.as_ptr() as isize as i64),
        _ => Err(format!(
            "mpv embed playback currently supports Windows HWND, X11 window, and macOS AppKit NSView hosts; {} video host support is not implemented yet",
            std::env::consts::OS
        )),
    }
}

#[cfg(windows)]
fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    Ok(i64::from(window))
}

#[cfg(not(windows))]
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    if window > i64::MAX as core::ffi::c_ulong {
        Err("Xlib window id is too large for mpv wid".to_string())
    } else {
        Ok(window as i64)
    }
}

#[cfg(windows)]
fn window_hwnd(window: &impl HasWindowHandle) -> Result<i64, String> {
    window_mpv_wid(window)
}

#[cfg(windows)]
fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
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
    fn accepts_supported_stream_urls_as_media_locations() {
        assert_eq!(
            validate_media_path("https://example.com/live.m3u8")
                .expect("https streams should be accepted")
                .to_string_lossy(),
            "https://example.com/live.m3u8"
        );
        assert_eq!(
            validate_media_path("rtsp://camera.local/stream")
                .expect("rtsp streams should be accepted")
                .to_string_lossy(),
            "rtsp://camera.local/stream"
        );
    }

    #[test]
    fn rejects_unsupported_stream_urls_as_media_locations() {
        let error = validate_media_path("file://C:/secret.mp4")
            .expect_err("unsafe stream protocols should be rejected");

        assert!(error.contains("unsupported media stream protocol"));
    }

    #[test]
    fn hls_manifest_urls_force_lavf_hls_demuxer() {
        assert!(is_hls_manifest_media_url(
            "https://ali-m-l.cztv.com/channels/lantian/channel010/1080p.m3u8"
        ));
        assert!(is_hls_manifest_media_url(
            "HTTPS://example.com/live/CHANNEL.M3U8?token=abc#frag"
        ));
        assert!(!is_hls_manifest_media_url("https://example.com/movie.mp4"));
        assert!(!is_hls_manifest_media_url("rtsp://example.com/live.m3u8"));

        assert_eq!(
            loadfile_args_for_media_path("https://example.com/live.m3u8", None)
                .expect("hls load options should be accepted"),
            vec![
                "https://example.com/live.m3u8".to_string(),
                "replace".to_string(),
                "-1".to_string(),
                "demuxer=+lavf,demuxer-lavf-format=hls".to_string()
            ]
        );
        assert_eq!(
            legacy_hls_loadfile_args_for_media_path("https://example.com/live.m3u8", None)
                .expect("legacy hls load options should be accepted"),
            vec![
                "https://example.com/live.m3u8".to_string(),
                "replace".to_string(),
                "demuxer=+lavf,demuxer-lavf-format=hls".to_string()
            ]
        );
        assert_eq!(
            loadfile_args_for_media_path("https://example.com/movie.mp4", None)
                .expect("plain media should be accepted"),
            vec![
                "https://example.com/movie.mp4".to_string(),
                "replace".to_string()
            ]
        );
    }

    #[test]
    fn plugin_load_options_extend_safe_mpv_loadfile_options() {
        let options: MpvLoadOptions = serde_json::from_value(serde_json::json!({
            "demuxer": "+lavf",
            "demuxer-lavf-format": "hls"
        }))
        .expect("load options should deserialize from plugin hook result");

        assert_eq!(
            loadfile_args_for_media_path("https://example.com/live.custom", Some(&options))
                .expect("safe plugin load options should be accepted"),
            vec![
                "https://example.com/live.custom".to_string(),
                "replace".to_string(),
                "-1".to_string(),
                "demuxer=+lavf,demuxer-lavf-format=hls".to_string()
            ]
        );
    }

    #[test]
    fn plugin_load_options_reject_unsafe_mpv_loadfile_options() {
        let unknown_key: MpvLoadOptions = serde_json::from_value(serde_json::json!({
            "script": "evil.lua"
        }))
        .expect("unknown key should deserialize before validation");
        assert!(
            loadfile_args_for_media_path("https://example.com/live.custom", Some(&unknown_key))
                .expect_err("unknown load option keys should be rejected")
                .contains("unsupported mpv load option")
        );

        let comma_value: MpvLoadOptions = serde_json::from_value(serde_json::json!({
            "demuxer": "+lavf,hls"
        }))
        .expect("comma value should deserialize before validation");
        assert!(
            loadfile_args_for_media_path("https://example.com/live.custom", Some(&comma_value))
                .expect_err("comma-separated option injection should be rejected")
                .contains("invalid mpv load option")
        );
    }

    #[test]
    fn builds_sanitized_capture_output_paths() {
        let directory = PathBuf::from("captures");
        let path = capture_output_path(
            &directory,
            "https://example.com/live stream.m3u8",
            42,
            "png",
        );

        assert_eq!(
            path,
            PathBuf::from("captures").join("openplayer-live_stream-42.png")
        );
    }

    #[test]
    fn normalizes_capture_screenshot_formats() {
        assert_eq!(normalize_capture_image_format(None).unwrap(), "png");
        assert_eq!(
            normalize_capture_image_format(Some("JPEG".to_string())).unwrap(),
            "jpg"
        );
        assert_eq!(
            normalize_capture_image_format(Some("webp".to_string())).unwrap(),
            "webp"
        );
        assert_eq!(
            normalize_capture_image_format(Some("bmp".to_string()))
                .expect_err("unsupported screenshot formats should be rejected"),
            "unsupported screenshot format: bmp"
        );
    }

    #[test]
    fn builds_sanitized_recording_output_paths() {
        let directory = PathBuf::from("recordings");
        let path = recording_output_path(&directory, "rtsp://camera.local/live stream", 42, "mp4");

        assert_eq!(
            path,
            PathBuf::from("recordings").join("openplayer-live_stream-42.mp4")
        );
    }

    #[test]
    fn normalizes_recording_container_formats() {
        assert_eq!(normalize_recording_container_format(None).unwrap(), "mp4");
        assert_eq!(
            normalize_recording_container_format(Some("MKV".to_string())).unwrap(),
            "mkv"
        );
        assert_eq!(
            normalize_recording_container_format(Some("ts".to_string())).unwrap(),
            "ts"
        );
        assert_eq!(
            normalize_recording_container_format(Some("avi".to_string()))
                .expect_err("unsupported recording formats should be rejected"),
            "unsupported recording format: avi"
        );
    }

    #[test]
    fn dump_cache_recordings_preserve_requested_container() {
        assert_eq!(
            recording_container_format_for_method(
                &MpvRecordingMethod::DumpCache {
                    start_position: 12.0
                },
                "mp4"
            ),
            "mp4"
        );
        assert_eq!(
            recording_container_format_for_method(
                &MpvRecordingMethod::DumpCache {
                    start_position: 12.0
                },
                "ts"
            ),
            "ts"
        );
    }

    #[test]
    fn stream_recordings_preserve_requested_container() {
        assert_eq!(
            recording_container_format_for_method(&MpvRecordingMethod::StreamRecord, "ts"),
            "ts"
        );
        assert_eq!(
            recording_container_format_for_method(&MpvRecordingMethod::StreamRecord, "mp4"),
            "mp4"
        );
    }

    #[test]
    fn local_recordings_use_cache_dump_with_short_preroll() {
        assert_eq!(
            recording_method_for_media_path("F:\\Movies\\clip.mp4", 12.5),
            MpvRecordingMethod::DumpCache {
                start_position: 7.5
            }
        );
        assert_eq!(
            recording_method_for_media_path("F:\\Movies\\clip.mp4", 2.5),
            MpvRecordingMethod::DumpCache {
                start_position: 0.0
            }
        );
    }

    #[test]
    fn http_network_recordings_use_cache_dump_with_short_preroll() {
        assert_eq!(
            recording_method_for_media_path("https://example.com/live.m3u8", 12.5),
            MpvRecordingMethod::DumpCache {
                start_position: 7.5
            }
        );
    }

    #[test]
    fn live_network_recordings_use_stream_record() {
        assert_eq!(
            recording_method_for_media_path("rtsp://camera.local/live", 12.5),
            MpvRecordingMethod::StreamRecord
        );
        assert_eq!(
            recording_method_for_media_path("rtmp://example.com/live", 12.5),
            MpvRecordingMethod::StreamRecord
        );
    }

    #[test]
    fn recording_dump_start_positions_include_bounded_preroll() {
        assert_eq!(recording_dump_start_position(8.0), 3.0);
        assert_eq!(recording_dump_start_position(3.0), 0.0);
        assert_eq!(recording_dump_start_position(f64::NAN), 0.0);
    }

    #[test]
    fn recording_start_time_args_are_finite_and_non_negative() {
        assert_eq!(
            recording_time_arg(1.25).expect("valid recording start"),
            "1.250"
        );
        assert_eq!(
            recording_time_arg(-4.5).expect("negative starts should clamp"),
            "0.000"
        );
        assert_eq!(
            recording_time_arg(f64::NAN).expect_err("invalid starts should fail"),
            "recording start time is invalid"
        );
    }

    #[test]
    fn rejects_empty_recording_outputs() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-empty-recording-{}-{}",
            std::process::id(),
            current_time_ms_for_capture()
        ));
        std::fs::create_dir_all(&directory).expect("temp recording directory should be created");
        let output_path = directory.join("empty.mp4");
        std::fs::write(&output_path, []).expect("empty recording file should be written");

        let error = ensure_recording_output_has_content(&output_path)
            .expect_err("empty recording outputs should fail");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("empty recording file"));
    }

    #[test]
    fn polling_empty_recording_outputs_does_not_delete_them() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-empty-recording-poll-{}-{}",
            std::process::id(),
            current_time_ms_for_capture()
        ));
        std::fs::create_dir_all(&directory).expect("temp recording directory should be created");
        let output_path = directory.join("empty.mp4");
        std::fs::write(&output_path, []).expect("empty recording file should be written");

        assert!(
            !recording_output_has_content(&output_path).expect("empty file should be readable")
        );
        assert!(output_path.exists());
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn accepts_custom_capture_directory_overrides() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-capture-directory-{}-{}",
            std::process::id(),
            current_time_ms_for_capture()
        ));
        let resolved =
            normalize_capture_directory_override(Some(directory.to_string_lossy().to_string()))
                .expect("custom capture directory should normalize");
        let _ = std::fs::remove_dir_all(&directory);

        assert_eq!(resolved, Some(directory));
    }

    #[test]
    fn rejects_file_capture_directory_overrides() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-capture-directory-file-{}-{}",
            std::process::id(),
            current_time_ms_for_capture()
        ));
        std::fs::create_dir_all(&directory).expect("temp capture directory should be created");
        let file_path = directory.join("not-a-directory.txt");
        std::fs::write(&file_path, b"fixture").expect("temp file should be written");

        let error =
            normalize_capture_directory_override(Some(file_path.to_string_lossy().to_string()))
                .expect_err("file capture directory overrides should be rejected");
        let _ = std::fs::remove_dir_all(&directory);

        assert!(error.contains("capture directory path is not a directory"));
    }

    #[test]
    #[cfg(windows)]
    fn encodes_win32_class_name_with_null_terminator() {
        let encoded = wide_null("STATIC");

        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded[..6], [83, 84, 65, 84, 73, 67]);
    }

    #[test]
    fn clamps_supported_playback_speed_range() {
        assert_eq!(normalize_playback_speed(0.1).unwrap(), MIN_PLAYBACK_SPEED);
        assert_eq!(normalize_playback_speed(1.25).unwrap(), 1.25);
        assert_eq!(normalize_playback_speed(8.0).unwrap(), MAX_PLAYBACK_SPEED);
        assert_eq!(
            normalize_playback_speed(f64::NAN).expect_err("nan should be rejected"),
            "invalid mpv playback speed"
        );
    }

    #[test]
    fn clamps_supported_subtitle_delay_range() {
        assert_eq!(normalize_subtitle_delay(-30.0).unwrap(), MIN_SUBTITLE_DELAY);
        assert_eq!(normalize_subtitle_delay(0.15).unwrap(), 0.15);
        assert_eq!(normalize_subtitle_delay(45.0).unwrap(), MAX_SUBTITLE_DELAY);
        assert_eq!(
            normalize_subtitle_delay(f64::NAN).expect_err("nan should be rejected"),
            "invalid mpv subtitle delay"
        );
    }

    #[test]
    fn maps_hardware_decoding_modes_to_mpv_hwdec_values() {
        assert_eq!(normalize_hwdec_mode("hardware").unwrap(), "auto-safe");
        assert_eq!(normalize_hwdec_mode("software").unwrap(), "no");
        assert_eq!(normalize_hwdec_mode("auto-safe").unwrap(), "auto-safe");
        assert_eq!(normalize_hwdec_mode("no").unwrap(), "no");
        assert_eq!(
            normalize_hwdec_mode("gpu-next").expect_err("unsupported modes should be rejected"),
            "invalid mpv hardware decoding mode"
        );
    }

    #[test]
    fn normalizes_initial_resume_positions() {
        assert_eq!(normalize_initial_resume_position(Some(42.0)), Some(42.0));
        assert_eq!(normalize_initial_resume_position(Some(0.0)), None);
        assert_eq!(normalize_initial_resume_position(Some(-1.0)), None);
        assert_eq!(normalize_initial_resume_position(Some(f64::NAN)), None);
        assert_eq!(normalize_initial_resume_position(None), None);
    }

    #[test]
    fn normalizes_initial_volume_before_media_load() {
        assert_eq!(normalize_initial_volume(None).unwrap(), DEFAULT_VOLUME);
        assert_eq!(normalize_initial_volume(Some(0.0)).unwrap(), 0.0);
        assert_eq!(normalize_initial_volume(Some(150.0)).unwrap(), 100.0);
        assert_eq!(
            normalize_initial_volume(Some(f64::NAN)).expect_err("nan volume should be rejected"),
            "invalid mpv volume"
        );
    }

    #[test]
    fn waits_for_initial_resume_seek_until_duration_and_seekability_are_ready() {
        assert_eq!(
            initial_resume_seek_readiness(120.0, 0.0, true),
            InitialResumeSeekReadiness::Ready
        );
        assert_eq!(
            initial_resume_seek_readiness(120.0, 600.0, false),
            InitialResumeSeekReadiness::Ready
        );
        assert_eq!(
            initial_resume_seek_readiness(120.0, 600.0, true),
            InitialResumeSeekReadiness::Ready
        );
    }

    #[test]
    fn waits_instead_of_skipping_when_early_duration_is_shorter_than_resume_target() {
        assert_eq!(
            initial_resume_seek_readiness(1800.0, 30.0, false),
            InitialResumeSeekReadiness::Wait
        );
        assert_eq!(
            initial_resume_seek_readiness(1800.0, 30.0, true),
            InitialResumeSeekReadiness::Ready
        );
    }

    #[test]
    fn treats_early_mpv_command_rejection_as_transient_resume_seek_failure() {
        assert!(is_transient_initial_resume_seek_error(
            &libmpv2::Error::Raw(libmpv2::mpv_error::Command)
        ));
        assert!(!is_transient_initial_resume_seek_error(
            &libmpv2::Error::Raw(libmpv2::mpv_error::Generic)
        ));
    }

    #[test]
    fn skips_initial_resume_seek_only_when_target_is_invalid() {
        assert_eq!(
            initial_resume_seek_readiness(0.0, 600.0, true),
            InitialResumeSeekReadiness::Skip
        );
        assert_eq!(
            initial_resume_seek_readiness(f64::NAN, 600.0, true),
            InitialResumeSeekReadiness::Skip
        );
    }

    #[test]
    fn discovers_same_stem_sidecar_subtitles() {
        let directory = std::env::temp_dir().join(format!(
            "openplayer-sidecars-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).expect("temp subtitle directory should be created");

        let media = directory.join("episode.mkv");
        std::fs::write(&media, b"media").expect("media fixture should be written");
        std::fs::write(directory.join("episode.srt"), b"subtitle")
            .expect("subtitle fixture should be written");
        std::fs::write(directory.join("episode.zh-CN.ass"), b"subtitle")
            .expect("language subtitle fixture should be written");
        std::fs::write(directory.join("episode.notes.txt"), b"notes")
            .expect("non-subtitle fixture should be written");
        std::fs::write(directory.join("other.srt"), b"subtitle")
            .expect("unrelated subtitle fixture should be written");

        let names: Vec<String> = discover_sidecar_subtitles(&media)
            .into_iter()
            .map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .expect("subtitle file name should be utf-8")
                    .to_string()
            })
            .collect();

        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(names, vec!["episode.srt", "episode.zh-CN.ass"]);
    }

    #[test]
    fn enables_real_audio_visualizer_for_audio_files_only() {
        assert!(is_likely_audio_path(Path::new("song.MP3")));
        assert!(is_likely_audio_path(Path::new("voice.amr")));
        assert!(is_likely_audio_path(Path::new("audiobook.m4b")));
        assert!(is_likely_audio_path(Path::new("sample.caf")));
        assert!(is_likely_audio_path(Path::new("album.track.flac")));
        assert!(is_likely_audio_path(Path::new("mix.opus")));
        assert!(!is_likely_audio_path(Path::new("movie.mp4")));
        assert!(!is_likely_audio_path(Path::new("clip.mkv")));
    }

    #[test]
    fn maps_track_kinds_to_mpv_properties() {
        assert_eq!(track_property_for_kind("audio").unwrap(), "aid");
        assert_eq!(track_property_for_kind("video").unwrap(), "vid");
        assert_eq!(track_property_for_kind("subtitle").unwrap(), "sid");
        assert_eq!(track_property_for_kind("sub").unwrap(), "sid");
        assert_eq!(
            track_property_for_kind("chapter").expect_err("unsupported kinds should be rejected"),
            "invalid mpv track kind"
        );
    }

    #[test]
    fn normalizes_plugin_owned_mpv_properties() {
        assert_eq!(
            normalize_plugin_mpv_property("sub-font-size", &serde_json::json!(52)).unwrap(),
            ("sub-font-size", PluginMpvPropertyValue::Number(52.0))
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-font", &serde_json::json!("Inter")).unwrap(),
            (
                "sub-font",
                PluginMpvPropertyValue::Text("Inter".to_string())
            )
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-color", &serde_json::json!("#78d5b3")).unwrap(),
            (
                "sub-color",
                PluginMpvPropertyValue::Text("#78d5b3".to_string())
            )
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(4)).unwrap(),
            ("sub-spacing", PluginMpvPropertyValue::Text("4".to_string()))
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(10)).unwrap(),
            (
                "sub-spacing",
                PluginMpvPropertyValue::Text("10".to_string())
            )
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-border-size", &serde_json::json!(2.5)).unwrap(),
            ("sub-outline-size", PluginMpvPropertyValue::Number(2.5))
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-shadow-offset", &serde_json::json!(1.5)).unwrap(),
            ("sub-shadow-offset", PluginMpvPropertyValue::Number(1.5))
        );
    }

    #[test]
    fn rejects_plugin_owned_mpv_properties_outside_allowlist() {
        assert_eq!(
            normalize_plugin_mpv_property("vf", &serde_json::json!("lavfi=[scale=2]"))
                .expect_err("plugins must not set arbitrary mpv properties"),
            "unsupported plugin mpv property: vf"
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-font-size", &serde_json::json!(999))
                .expect_err("subtitle font size outside the allowed range should be rejected"),
            "invalid plugin subtitle font size"
        );
        assert_eq!(
            normalize_plugin_mpv_property("sub-spacing", &serde_json::json!(11))
                .expect_err("subtitle spacing above mpv's stable range should be rejected"),
            "invalid plugin subtitle spacing"
        );
    }

    #[test]
    fn plugin_subtitle_style_properties_force_ass_overrides() {
        assert!(plugin_subtitle_style_requires_ass_override("sub-font-size"));
        assert!(plugin_subtitle_style_requires_ass_override("sub-spacing"));
        assert!(!plugin_subtitle_style_requires_ass_override(
            "sub-line-spacing"
        ));
        assert!(!plugin_subtitle_style_requires_ass_override("sub-delay"));
    }

    #[test]
    fn subtitle_spacing_writes_only_stable_mpv_property() {
        assert_eq!(
            plugin_mpv_property_write_targets("sub-line-spacing"),
            &[] as &[&str]
        );
        assert_eq!(
            plugin_mpv_property_write_targets("sub-spacing"),
            &["sub-spacing"]
        );
    }

    #[test]
    fn prepares_numeric_locale_for_libmpv_initialization() {
        assert!(prepare_libmpv_numeric_locale().is_ok());
    }

    #[test]
    fn linux_video_output_falls_back_to_x11_when_dri_render_node_is_missing() {
        let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
            override_vo: None,
            override_gpu_context: None,
            override_hwdec: None,
            has_dri_render_node: false,
            virtual_drm_driver: false,
        });

        assert_eq!(
            config,
            MpvVideoOutputConfig {
                vo: Some("x11".to_string()),
                gpu_context: None,
                hwdec: "no".to_string(),
            }
        );
    }

    #[test]
    fn linux_video_output_falls_back_to_x11_for_virtual_drm_drivers() {
        let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
            override_vo: None,
            override_gpu_context: None,
            override_hwdec: None,
            has_dri_render_node: true,
            virtual_drm_driver: true,
        });

        assert_eq!(
            config,
            MpvVideoOutputConfig {
                vo: Some("x11".to_string()),
                gpu_context: None,
                hwdec: "no".to_string(),
            }
        );
    }

    #[test]
    fn linux_video_output_uses_x11egl_when_dri_render_node_is_available() {
        let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
            override_vo: None,
            override_gpu_context: None,
            override_hwdec: None,
            has_dri_render_node: true,
            virtual_drm_driver: false,
        });

        assert_eq!(
            config,
            MpvVideoOutputConfig {
                vo: Some("gpu".to_string()),
                gpu_context: Some("x11egl".to_string()),
                hwdec: "auto-safe".to_string(),
            }
        );
    }

    #[test]
    fn linux_video_output_allows_field_vo_override() {
        let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
            override_vo: Some("x11"),
            override_gpu_context: None,
            override_hwdec: None,
            has_dri_render_node: true,
            virtual_drm_driver: false,
        });

        assert_eq!(
            config,
            MpvVideoOutputConfig {
                vo: Some("x11".to_string()),
                gpu_context: None,
                hwdec: "no".to_string(),
            }
        );
    }

    #[test]
    fn linux_video_output_allows_gpu_context_and_hwdec_overrides() {
        let config = resolve_linux_video_output_config(LinuxVideoOutputEnvironment {
            override_vo: Some("gpu"),
            override_gpu_context: Some("x11"),
            override_hwdec: Some("no"),
            has_dri_render_node: false,
            virtual_drm_driver: true,
        });

        assert_eq!(
            config,
            MpvVideoOutputConfig {
                vo: Some("gpu".to_string()),
                gpu_context: Some("x11".to_string()),
                hwdec: "no".to_string(),
            }
        );
    }

    #[test]
    fn identifies_known_virtual_linux_drm_drivers() {
        assert!(is_virtual_linux_drm_driver("bochs-drm"));
        assert!(is_virtual_linux_drm_driver("QXL"));
        assert!(is_virtual_linux_drm_driver("virtio_gpu"));
        assert!(!is_virtual_linux_drm_driver("i915"));
        assert!(!is_virtual_linux_drm_driver("amdgpu"));
    }

    #[test]
    fn forwards_mpv_video_diagnostic_log_messages() {
        assert!(is_mpv_video_diagnostic_log(
            "warn",
            "vo/gpu",
            "libEGL warning: DRI3 error: Could not get DRI3 device"
        ));
        assert!(is_mpv_video_diagnostic_log(
            "info",
            "cplayer",
            "VO: [x11] 1280x720 yuv420p"
        ));
        assert!(is_mpv_video_diagnostic_log(
            "v",
            "vd",
            "Trying hardware decoding via vaapi"
        ));
        assert!(!is_mpv_video_diagnostic_log(
            "info",
            "cplayer",
            "Playing: sample.mp4"
        ));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_video_output_uses_libmpv_render_api_vo() {
        let config = platform_video_output_config();

        assert_eq!(
            config,
            MpvVideoOutputConfig {
                vo: Some("libmpv".to_string()),
                gpu_context: None,
                hwdec: "auto-safe".to_string(),
            }
        );
    }

    #[test]
    fn maps_x11_window_handles_to_mpv_wid_values() {
        let xlib = RawWindowHandle::Xlib(raw_window_handle::XlibWindowHandle::new(42));
        assert_eq!(mpv_wid_from_raw_window_handle(xlib).unwrap(), 42);

        let xcb_window = std::num::NonZeroU32::new(84).expect("fixture window id is non-zero");
        let xcb = RawWindowHandle::Xcb(raw_window_handle::XcbWindowHandle::new(xcb_window));
        assert_eq!(mpv_wid_from_raw_window_handle(xcb).unwrap(), 84);
    }

    #[test]
    fn rejects_wayland_until_native_host_exists() {
        let surface = std::ptr::NonNull::dangling();
        let handle = RawWindowHandle::Wayland(raw_window_handle::WaylandWindowHandle::new(surface));

        assert_eq!(
            mpv_wid_from_raw_window_handle(handle).expect_err("Wayland does not support mpv wid"),
            "mpv embed playback currently supports Windows HWND and X11 window hosts; Wayland video host support is not implemented yet"
        );
    }

    #[test]
    fn wall_tile_requests_accept_rtsp_rtmp_http_and_hls_streams() {
        for url in [
            "rtsp://example.test/live/one",
            "rtmp://example.test/live/two",
            "http://example.test/live/three.mp4",
            "https://example.test/live/four.m3u8",
            "rtsp://[240e:39f:3a6:6f70:cc97:60e7:d590:7281]:8554/webm_rtsp_1",
            "rtmp://[240e:39f:3a6:6f70:cc97:60e7:d590:7281]:19350/webm_rtmp",
        ] {
            let tile = MpvWallTileRequest {
                id: "tile-1".to_string(),
                url: url.to_string(),
                title: Some("Camera".to_string()),
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 0.5,
                muted: Some(true),
            };

            assert_eq!(normalize_wall_tile_request(tile).unwrap().url, url);
        }
    }

    #[test]
    fn wall_tile_fraction_layout_maps_to_parent_pixels() {
        let rect = normalize_wall_tile_rect(0.25, 0.5, 0.5, 0.25).unwrap();
        let layout = wall_tile_rect_to_video_host_rect(1920, 1080, rect);

        assert_eq!(
            layout,
            VideoHostRect {
                x: 480,
                y: 540,
                width: 960,
                height: 270,
            }
        );
    }

    #[test]
    fn wall_open_initial_snapshots_cover_every_tile_before_players_start() {
        let tiles = normalize_wall_tile_requests(vec![
            MpvWallTileRequest {
                id: "rtsp-one".to_string(),
                url: "rtsp://example.test/live/one".to_string(),
                title: Some("One".to_string()),
                x: 0.0,
                y: 0.0,
                width: 0.5,
                height: 0.5,
                muted: Some(true),
            },
            MpvWallTileRequest {
                id: "rtmp-two".to_string(),
                url: "rtmp://example.test/live/two".to_string(),
                title: Some("Two".to_string()),
                x: 0.5,
                y: 0.0,
                width: 0.5,
                height: 0.5,
                muted: Some(true),
            },
        ])
        .unwrap();

        let snapshots = wall_initial_snapshots(&tiles);

        assert_eq!(snapshots.len(), 2);
        assert!(
            snapshots
                .iter()
                .all(|snapshot| snapshot.status == "loading")
        );
        assert_eq!(snapshots[0].id, "rtsp-one");
        assert_eq!(snapshots[1].id, "rtmp-two");
    }

    #[test]
    fn wall_live_status_keeps_terminal_and_loading_states_stable() {
        assert_eq!(wall_live_status(true, false, false), "ended");
        assert_eq!(wall_live_status(false, true, false), "paused");
        assert_eq!(wall_live_status(false, false, true), "loading");
        assert_eq!(wall_live_status(false, false, false), "playing");
    }

    #[test]
    fn wall_bitrate_prefers_track_bitrates_and_falls_back_to_raw_input_rate() {
        assert_eq!(
            combine_wall_bitrate(Some(4_000_000.0), Some(160_000.0), Some(100_000.0)),
            Some(4_160_000.0)
        );
        assert_eq!(
            combine_wall_bitrate(None, None, Some(250_000.0)),
            Some(2_000_000.0)
        );
        assert_eq!(combine_wall_bitrate(Some(0.0), None, Some(-1.0)), None);
    }

    #[test]
    fn wall_osd_formats_buffer_in_milliseconds() {
        assert_eq!(format_wall_buffer_millis(Some(0.021)), "21 ms");
        assert_eq!(format_wall_buffer_millis(Some(1.234)), "1234 ms");
        assert_eq!(format_wall_buffer_millis(None), "-- ms");
    }

    #[test]
    fn wall_osd_formats_bitrate_compactly() {
        assert_eq!(format_wall_bitrate(Some(2_500_000.0)), "2.5 Mbps");
        assert_eq!(format_wall_bitrate(Some(640_000.0)), "640 Kbps");
        assert_eq!(format_wall_bitrate(None), "--");
    }

    #[test]
    fn wall_request_ids_are_unique_per_generation_and_tile() {
        assert_eq!(wall_request_id(7, 0), 7_001);
        assert_eq!(wall_request_id(7, 1), 7_002);
        assert_ne!(wall_request_id(7, 0), wall_request_id(8, 0));
    }

    #[test]
    #[cfg(windows)]
    fn wall_open_reuses_same_tile_set_without_resetting_generation() {
        let state = MpvWallState::default();
        let tiles = normalize_wall_tile_requests(vec![MpvWallTileRequest {
            id: "rtsp-one".to_string(),
            url: "rtsp://example.test/live/one".to_string(),
            title: Some("One".to_string()),
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            muted: Some(true),
        }])
        .unwrap();

        let generation = state.next_generation().unwrap();
        state
            .replace_opening_state(wall_initial_snapshots(&tiles))
            .unwrap();

        assert!(state.can_reuse_open_wall(&tiles).unwrap());
        assert_eq!(state.current_generation().unwrap(), generation);
    }

    #[test]
    #[cfg(windows)]
    fn wall_starting_guard_prevents_duplicate_tile_starts() {
        let state = MpvWallState::default();
        let generation = state.next_generation().unwrap();

        assert!(state.mark_tile_starting(generation, "camera-1").unwrap());
        assert!(!state.mark_tile_starting(generation, "camera-1").unwrap());
        state.clear_tile_starting("camera-1").unwrap();
        assert!(state.mark_tile_starting(generation, "camera-1").unwrap());
    }

    #[test]
    #[cfg(windows)]
    fn wall_take_players_clears_players_without_resetting_generation() {
        let state = MpvWallState::default();
        let generation = state.next_generation().unwrap();

        let players = state.take_players().unwrap();

        assert!(players.is_empty());
        assert_eq!(state.current_generation().unwrap(), generation);
    }

    #[test]
    #[cfg(windows)]
    fn reserves_web_controls_outside_native_video_host() {
        let rect = video_host_rect(1280, 720);

        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 1280);
        assert_eq!(rect.height, 720);
    }
}
