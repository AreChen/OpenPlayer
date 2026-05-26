use std::time::Duration;

#[cfg(windows)]
pub(super) const VIDEO_HOST_TOP_RESERVE: i32 = 0;
#[cfg(windows)]
pub(super) const VIDEO_HOST_BOTTOM_RESERVE: i32 = 0;
pub(super) const END_OF_MEDIA_SNAP_TOLERANCE_SECONDS: f64 = 0.5;
pub(super) const FRAME_STEP_SETTLE_INTERVAL: Duration = Duration::from_millis(6);
pub(super) const FRAME_STEP_SETTLE_TIMEOUT: Duration = Duration::from_millis(180);
pub(super) const FRAME_STEP_PAUSE_GUARD: Duration = Duration::from_millis(350);
pub(super) const INITIAL_RESUME_SEEK_TIMEOUT: Duration = Duration::from_millis(8000);
pub(super) const INITIAL_RESUME_SEEK_EVENT_WAIT: Duration = Duration::from_millis(80);
pub(super) const INITIAL_RESUME_SEEK_SETTLE_TIMEOUT: Duration = Duration::from_millis(750);
pub(super) const INITIAL_RESUME_SEEK_TOLERANCE_SECONDS: f64 = 1.0;
pub(super) const MAIN_PLAYER_ASYNC_LOAD_REQUEST_ID: u64 = 1;
pub(super) const RECORDING_OUTPUT_READY_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const RECORDING_DUMP_PREROLL_SECONDS: f64 = 5.0;
pub(super) const DEFAULT_VOLUME: f64 = 82.0;
pub(super) const MIN_PLAYBACK_SPEED: f64 = 0.25;
pub(super) const MAX_PLAYBACK_SPEED: f64 = 4.0;
pub(super) const MIN_SUBTITLE_DELAY: f64 = -10.0;
pub(super) const MAX_SUBTITLE_DELAY: f64 = 10.0;
pub(super) const MAX_TRACKS: i64 = 128;
pub(super) const SUPPORTED_SUBTITLE_EXTENSIONS: &[&str] = &["ass", "srt", "ssa", "sub", "vtt"];
pub(super) const AUDIO_VISUALIZER_EXTENSIONS: &[&str] = &[
    "aac", "ac3", "adts", "aif", "aifc", "aiff", "alac", "amr", "ape", "au", "awb", "caf", "dff",
    "dsf", "dts", "dtshd", "eac3", "flac", "gsm", "m4a", "m4b", "m4r", "mka", "mlp", "mp1", "mp2",
    "mp3", "mpa", "mpc", "oga", "ogg", "opus", "ra", "snd", "spx", "tak", "tta", "voc", "wav",
    "weba", "wma", "wv",
];
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) const OPENPLAYER_MPV_VO_ENV: &str = "OPENPLAYER_MPV_VO";
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) const OPENPLAYER_MPV_GPU_CONTEXT_ENV: &str = "OPENPLAYER_MPV_GPU_CONTEXT";
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(super) const OPENPLAYER_MPV_HWDEC_ENV: &str = "OPENPLAYER_MPV_HWDEC";
pub(super) const MAX_MPV_WALL_TILES: usize = 16;
pub(super) const MIN_MPV_WALL_TILE_RATIO: f64 = 0.02;
pub(super) const MPV_WALL_TILE_START_STAGGER: Duration = Duration::from_millis(120);
pub(super) const MPV_WALL_EVENT_DRAIN_LIMIT: usize = 32;
#[cfg(windows)]
pub(super) const MPV_WALL_TILE_CORNER_RADIUS: i32 = 10;
#[cfg(windows)]
pub(super) const MPV_WALL_TILE_BORDER_INSET: i32 = 1;
