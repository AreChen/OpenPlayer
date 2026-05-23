use std::{
    ffi::CString,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use libmpv2::{events::Event, mpv_end_file_reason};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Serialize;
use tauri::{State, Window};
#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetClientRect, HWND_TOP, SW_SHOW, SWP_NOACTIVATE,
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

#[cfg(windows)]
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

struct MpvEmbedPlayer {
    mpv: libmpv2::Mpv,
    host: MpvVideoHost,
    path: String,
    volume: f64,
    ended: bool,
    force_paused_until: Option<Instant>,
}

#[cfg(windows)]
struct MpvVideoHost {
    parent_hwnd: isize,
    hwnd: isize,
}

#[cfg(not(windows))]
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
    let host = MpvVideoHost::new(window)?;
    let wid = host.wid();
    let mpv = create_embed_player(wid)?;
    let path_text = path.to_string_lossy().to_string();

    configure_audio_visualizer(&mpv, &path);
    mpv.command("loadfile", &[&path_text, "replace"])
        .map_err(|error| format!("mpv loadfile failed: {error}"))?;
    load_sidecar_subtitles(&mpv, &path);

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
        force_paused_until: None,
    };
    let snapshot = next_player.snapshot(wid, "playing");
    *player = Some(next_player);

    Ok(snapshot)
}

#[tauri::command]
pub fn mpv_embed_play(state: State<'_, MpvEmbedState>) -> Result<MpvEmbedSnapshot, String> {
    with_player(&state, |player| {
        player.force_paused_until = None;
        player.ended = false;
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
        player.force_paused_until = None;
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
        player.force_paused_until = None;
        player.ended = false;
        player
            .mpv
            .command("seek", &[&position.to_string(), "absolute"])
            .map_err(|error| format!("mpv seek failed: {error}"))?;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_frame_step(state: State<'_, MpvEmbedState>) -> Result<MpvEmbedSnapshot, String> {
    frame_step(&state, "frame-step")
}

#[tauri::command]
pub fn mpv_embed_frame_back_step(
    state: State<'_, MpvEmbedState>,
) -> Result<MpvEmbedSnapshot, String> {
    frame_step(&state, "frame-back-step")
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
pub fn mpv_embed_set_speed(
    state: State<'_, MpvEmbedState>,
    speed: f64,
) -> Result<MpvEmbedSnapshot, String> {
    let speed = normalize_playback_speed(speed)?;

    with_player(&state, |player| {
        player
            .mpv
            .set_property("speed", speed)
            .map_err(|error| format!("mpv speed failed: {error}"))?;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_set_hwdec(
    state: State<'_, MpvEmbedState>,
    mode: String,
) -> Result<MpvEmbedSnapshot, String> {
    let hwdec = normalize_hwdec_mode(&mode)?;

    with_player(&state, |player| {
        player
            .mpv
            .set_property("hwdec", hwdec)
            .map_err(|error| format!("mpv hardware decoding switch failed: {error}"))?;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_set_loop_file(
    state: State<'_, MpvEmbedState>,
    enabled: bool,
) -> Result<MpvEmbedSnapshot, String> {
    with_player(&state, |player| {
        player
            .mpv
            .set_property("loop-file", if enabled { "inf" } else { "no" })
            .map_err(|error| format!("mpv loop-file mode failed: {error}"))?;
        if enabled {
            player.ended = false;
        }
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_set_subtitle_delay(
    state: State<'_, MpvEmbedState>,
    delay: f64,
) -> Result<MpvEmbedSnapshot, String> {
    let delay = normalize_subtitle_delay(delay)?;

    with_player(&state, |player| {
        player
            .mpv
            .set_property("sub-delay", delay)
            .map_err(|error| format!("mpv subtitle delay failed: {error}"))?;
        Ok(player.snapshot(0, "playing"))
    })
}

#[tauri::command]
pub fn mpv_embed_select_track(
    state: State<'_, MpvEmbedState>,
    kind: String,
    track_id: Option<i64>,
) -> Result<MpvEmbedSnapshot, String> {
    let property = track_property_for_kind(&kind)?;
    if track_id.is_some_and(|id| id <= 0) {
        return Err("invalid mpv track id".to_string());
    }

    with_player(&state, |player| {
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
}

#[tauri::command]
pub fn mpv_embed_add_subtitle(
    state: State<'_, MpvEmbedState>,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    let path = validate_subtitle_path(&path)?;
    let path_text = path.to_string_lossy().to_string();

    with_player(&state, |player| {
        player
            .mpv
            .command("sub-add", &[&path_text, "select"])
            .map_err(|error| format!("mpv subtitle load failed: {error}"))?;
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

    Ok(player.as_mut().map(|player| player.snapshot(0, "playing")))
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

impl MpvEmbedPlayer {
    fn snapshot(&mut self, hwnd: i64, fallback_status: &str) -> MpvEmbedSnapshot {
        let _ = self.host.resize();
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
            match event {
                Ok(Event::EndFile(mpv_end_file_reason::Eof)) => {
                    self.ended = true;
                }
                Ok(Event::StartFile | Event::Seek | Event::PlaybackRestart) => {
                    self.ended = false;
                }
                Ok(Event::LogMessage {
                    prefix,
                    level,
                    text,
                    ..
                }) => {
                    log_mpv_video_diagnostic(prefix, level, text);
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
    prepare_libmpv_numeric_locale()?;
    let video_output_config = platform_video_output_config();
    log_selected_mpv_video_output_config(&video_output_config);

    let mpv = libmpv2::Mpv::with_initializer(|initializer| {
        initializer.set_option("wid", hwnd)?;
        configure_native_video_output(&initializer, &video_output_config)?;
        initializer.set_option("input-default-bindings", false)?;
        initializer.set_option("input-vo-keyboard", false)?;
        initializer.set_option("keep-open", true)?;
        initializer.set_option("load-scripts", true)?;
        initializer.set_option("osc", false)?;
        Ok(())
    })
    .map_err(|error| format!("mpv embed init failed: {error}"))?;

    request_mpv_log_messages(&mpv);

    Ok(mpv)
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

#[cfg(not(target_os = "linux"))]
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
        if let Err(error) = position_video_host(hwnd, layout) {
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
        })
    }

    #[cfg(not(windows))]
    fn new(window: &impl HasWindowHandle) -> Result<Self, String> {
        Ok(Self {
            wid: window_mpv_wid(window)?,
        })
    }

    #[cfg(windows)]
    fn wid(&self) -> i64 {
        self.hwnd as i64
    }

    #[cfg(not(windows))]
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

    #[cfg(not(windows))]
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

fn window_mpv_wid(window: &impl HasWindowHandle) -> Result<i64, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;

    mpv_wid_from_raw_window_handle(handle.as_raw())
}

fn mpv_wid_from_raw_window_handle(handle: RawWindowHandle) -> Result<i64, String> {
    match handle {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as i64),
        RawWindowHandle::Xlib(handle) if handle.window > 0 => xlib_window_to_mpv_wid(handle.window),
        RawWindowHandle::Xcb(handle) => Ok(i64::from(handle.window.get())),
        RawWindowHandle::Wayland(_) => Err(
            "mpv embed playback currently supports Windows HWND and X11 window hosts; Wayland video host support is not implemented yet"
                .to_string(),
        ),
        RawWindowHandle::AppKit(_) => Err(
            "mpv embed playback currently supports Windows HWND and X11 window hosts; macOS AppKit video host support is not implemented yet"
                .to_string(),
        ),
        _ => Err(format!(
            "mpv embed playback currently supports Windows HWND and X11 window hosts; {} video host support is not implemented yet",
            std::env::consts::OS
        )),
    }
}

#[cfg(windows)]
fn xlib_window_to_mpv_wid(window: core::ffi::c_ulong) -> Result<i64, String> {
    Ok(i64::from(window))
}

#[cfg(not(windows))]
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
    #[cfg(windows)]
    fn reserves_web_controls_outside_native_video_host() {
        let rect = video_host_rect(1280, 720);

        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 1280);
        assert_eq!(rect.height, 720);
    }
}
