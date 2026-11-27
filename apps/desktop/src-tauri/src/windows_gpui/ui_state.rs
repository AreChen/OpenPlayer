use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Instant,
};

use gpui::SharedString;
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub(crate) struct UiState {
    pub(super) labels: Arc<Value>,
    pub(super) appearance: Option<HashMap<String, String>>,
    pub(super) media: Option<NamedItem>,
    pub(super) audio_only: bool,
    pub(super) audio_codec: Option<String>,
    pub(super) loaded: bool,
    pub(super) loading: bool,
    pub(super) playing: bool,
    pub(super) advancing: bool,
    pub(super) sampled_at: Option<Instant>,
    pub(super) timeline_generation: u64,
    pub(super) fps: Option<f64>,
    pub(super) time_display_frames: bool,
    pub(super) position: f64,
    pub(super) duration: f64,
    pub(super) queued_previous: bool,
    pub(super) queued_next: bool,
    pub(super) muted: bool,
    pub(super) volume: f64,
    pub(super) speed: f64,
    pub(super) speed_label: String,
    pub(super) speeds: Vec<Speed>,
    pub(super) software_decode: bool,
    pub(super) panel: Option<MediaPanel>,
    pub(super) playlist: bool,
    pub(super) loop_mode: String,
    pub(super) loops: Vec<LoopMode>,
    pub(super) tracks: Vec<Track>,
    pub(super) video_fill: bool,
    pub(super) subtitle_delay: f64,
    pub(super) queue: Vec<NamedItem>,
    pub(super) queue_index: Option<usize>,
    pub(super) history: Vec<HistoryEntry>,
    pub(super) context: Option<ContextMenu>,
    pub(super) settings: Option<String>,
    pub(super) themes: Vec<Theme>,
    pub(super) theme_id: Option<String>,
    pub(super) accent_override: Option<String>,
    pub(super) accents: Vec<String>,
    pub(super) language: String,
    pub(super) languages: Vec<LabeledId>,
    pub(super) incognito: bool,
    pub(super) quiet_keyboard: bool,
    pub(super) titlebar_opacity: f64,
    pub(super) transport_opacity: f64,
    pub(super) shortcut_bindings: BTreeMap<String, String>,
    pub(super) shortcut_rows: Vec<Shortcut>,
    pub(super) recording_shortcut: Option<String>,
    pub(super) version: Option<Version>,
    pub(super) stream: Option<StreamDialog>,
    pub(super) error: Option<String>,
    pub(super) feedback: Option<String>,
}

impl UiState {
    pub fn label(&self, key: &str) -> SharedString {
        key.split('.')
            .try_fold(self.labels.as_ref(), |value, part| value.get(part))
            .and_then(Value::as_str)
            .unwrap_or_else(|| key.rsplit('.').next().unwrap_or_default())
            .to_owned()
            .into()
    }
}

#[derive(Clone, Debug)]
pub(super) struct NamedItem {
    pub name: String,
}

#[derive(Clone, Debug)]
pub(super) struct HistoryEntry {
    pub name: String,
    pub position: f64,
    pub duration: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MediaPanel {
    Speed,
    Tracks,
    Loop,
}

#[derive(Clone, Debug)]
pub(super) struct Speed {
    pub value: f64,
    pub label: String,
}

#[derive(Clone, Debug)]
pub(super) struct LoopMode {
    pub mode: String,
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Track {
    pub id: i32,
    pub kind: String,
    pub title: Option<String>,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub selected: bool,
}

#[derive(Clone, Debug)]
pub(super) struct LabeledId {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug)]
pub(super) struct Theme {
    pub id: String,
    pub name: String,
    pub accent: String,
}

#[derive(Clone, Debug)]
pub(super) struct ContextMenu {
    pub position: MenuPosition,
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct MenuPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub(super) struct MenuItem {
    pub id: String,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub shortcut: Option<String>,
    pub disabled: bool,
    pub divider: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Shortcut {
    pub id: String,
    pub label: String,
    pub binding: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct Version {
    pub name: String,
    pub version: String,
    pub license: String,
    pub repository: String,
    pub releases_url: String,
}

#[derive(Clone, Debug)]
pub(super) struct StreamDialog {
    pub error: Option<String>,
    pub history: Vec<NamedItem>,
}
