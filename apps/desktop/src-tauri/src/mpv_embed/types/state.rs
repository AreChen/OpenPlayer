use std::{collections::BTreeMap, sync::Mutex, time::Instant};

#[cfg(windows)]
use std::{collections::BTreeSet, sync::Arc};

#[cfg(target_os = "macos")]
use super::MacosMpvRenderContext;
use super::{
    MpvRecordingSession, MpvVideoHost, MpvWallTileRect, MpvWallTileSnapshot, VideoHostRect,
};

#[derive(Default)]
pub struct MpvEmbedState {
    pub(crate) player: Mutex<Option<MpvEmbedPlayer>>,
}

#[derive(Default)]
pub struct MpvWallState {
    #[cfg(windows)]
    pub(crate) players: Mutex<BTreeMap<String, MpvWallPlayer>>,
    #[cfg(windows)]
    pub(crate) starting: Mutex<BTreeSet<String>>,
    pub(crate) statuses: Mutex<BTreeMap<String, MpvWallTileSnapshot>>,
    pub(crate) generation: Mutex<u64>,
}

pub(crate) struct MpvEmbedPlayer {
    #[cfg(target_os = "macos")]
    pub(crate) _render_context: MacosMpvRenderContext,
    pub(crate) mpv: libmpv2::Mpv,
    pub(crate) host: MpvVideoHost,
    pub(crate) path: String,
    pub(crate) volume: f64,
    pub(crate) video_fill: bool,
    pub(crate) ended: bool,
    pub(crate) force_paused_until: Option<Instant>,
    pub(crate) recording: Option<MpvRecordingSession>,
}

#[cfg(windows)]
pub(crate) struct MpvWallPlayer {
    pub(crate) id: String,
    pub(crate) url: String,
    pub(crate) title: Option<String>,
    pub(crate) rect: MpvWallTileRect,
    pub(crate) mpv: Arc<libmpv2::Mpv>,
    pub(crate) host: MpvVideoHost,
}

#[cfg(windows)]
#[derive(Clone)]
pub(crate) struct MpvWallHostLayout {
    pub(crate) id: String,
    pub(crate) layout: VideoHostRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InitialResumeSeekReadiness {
    Ready,
    Wait,
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MpvEventEffect {
    None,
    Active,
    Ended,
}
