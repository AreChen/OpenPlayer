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

mod capture_recording;
mod media;
mod plugin_properties;
mod video_host;
mod video_output;
mod wall;

use capture_recording::*;
use media::*;
use plugin_properties::*;
#[cfg(test)]
use video_host::*;
use video_output::*;
use wall::*;

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

#[cfg(test)]
mod tests;
