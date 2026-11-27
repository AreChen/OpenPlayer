mod media;
mod preferences;
mod ui;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use libmpv2::{Mpv, events::Event};
use libmpv2_sys::mpv_end_file_reason_MPV_END_FILE_REASON_EOF;
use serde_json::Value;

use crate::{
    media_paths::collect_media_files_from_paths,
    playback_store::{
        MediaPlaybackSettingsUpdate, NetworkStreamHistoryEntry, NetworkStreamHistoryUpdate,
        PlaybackHistoryEntry, PlaybackHistoryUpdate, PlaybackSettings, PlaybackSettingsUpdate,
        PlaybackStore, network_stream,
    },
};

use self::preferences::{Preferences, storage_directory};
use super::{
    browser_options::BrowserOptions,
    ui_state::{HistoryEntry, NamedItem, Track, UiState},
};

const SETTINGS_DELAY: Duration = Duration::from_millis(350);
const SEEK_SETTLE: Duration = Duration::from_millis(450);
const HISTORY_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct Media {
    source: String,
    name: String,
    network: bool,
}

impl Media {
    fn local(source: String) -> Self {
        let name = Path::new(&source).file_name().map_or_else(
            || source.clone(),
            |name| name.to_string_lossy().into_owned(),
        );
        Self {
            source,
            name,
            network: false,
        }
    }

    fn item(&self) -> NamedItem {
        NamedItem {
            name: self.name.clone(),
        }
    }
}

struct PendingSeek {
    position: f64,
    requested: Instant,
}

pub(super) struct Controller {
    mpv: Option<Arc<Mpv>>,
    pending_resume: Option<f64>,
    ui: UiState,
    queue: Vec<Media>,
    current: Option<Media>,
    history: Arc<Vec<PlaybackHistoryEntry>>,
    stream_history: Arc<Vec<NetworkStreamHistoryEntry>>,
    store_path: PathBuf,
    preferences: Preferences,
    favorite_directories: Vec<PathBuf>,
    favorite_media: Vec<PathBuf>,
    media_browser_options: BrowserOptions,
    last_media_directory: Option<PathBuf>,
    playback: PlaybackSettings,
    pending_settings: PlaybackSettingsUpdate,
    settings_due: Option<Instant>,
    last_history: Instant,
    pending_seek: Option<PendingSeek>,
    eof_handled: bool,
    sidecar_tracks: Vec<i64>,
}

impl Controller {
    pub(super) fn new(startup_media: Vec<PathBuf>) -> Self {
        let directory = storage_directory();
        let store_path = directory.join("playback-history.redb");
        let preferences = Preferences::new(directory.join("openplayer-settings.redb"));
        let mut controller = Self {
            mpv: None,
            pending_resume: None,
            ui: UiState::default(),
            queue: Vec::new(),
            current: None,
            history: Arc::default(),
            stream_history: Arc::default(),
            store_path,
            preferences,
            favorite_directories: Vec::new(),
            favorite_media: Vec::new(),
            media_browser_options: BrowserOptions::default(),
            last_media_directory: None,
            playback: PlaybackSettings::default(),
            pending_settings: PlaybackSettingsUpdate::default(),
            settings_due: None,
            last_history: Instant::now(),
            pending_seek: None,
            eof_handled: false,
            sidecar_tracks: Vec::new(),
        };
        if let Err(error) = controller.restore_settings() {
            controller.report_error(error);
        }
        let paths = startup_media
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        if let Err(error) = controller.open_paths(paths, false) {
            controller.report_error(error);
        }
        controller
    }

    pub(super) fn awaiting_playback_core(&self) -> bool {
        self.pending_resume.is_some()
    }

    pub(super) fn attach_playback_core(&mut self, mpv: Arc<Mpv>) -> Result<()> {
        let media = self.current.as_ref().context("no pending media to play")?;
        let resume = self.pending_resume.context("no pending playback request")?;
        mpv.set_property("speed", self.playback.playback_speed)?;
        mpv.set_property("hwdec", self.hardware_decode())?;
        mpv.set_property("mute", self.ui.muted)?;
        self.apply_video_layout(&mpv)?;
        self.load_file(&mpv, media, resume)?;
        self.mpv = Some(mpv);
        self.pending_resume = None;
        Ok(())
    }

    pub(super) fn playback_core_failed(&mut self, error: anyhow::Error) {
        self.clear_media();
        self.report_error(error);
    }

    pub(super) fn should_release_playback_core(&self) -> bool {
        self.mpv.is_some() && self.current.is_none()
    }

    pub(super) fn detach_playback_core(&mut self) {
        debug_assert!(self.should_release_playback_core());
        self.mpv = None;
    }

    pub(super) fn snapshot(&mut self) -> Arc<UiState> {
        let now = Instant::now();
        if let Err(error) = self.refresh(now) {
            self.report_error(error);
        }
        Arc::new(self.ui.clone())
    }

    pub(super) fn dispatch(&mut self, kind: &str, value: Value) {
        if let Err(error) = self.action(kind, value) {
            if kind == "submitStream" {
                self.stream_error(format!("{error:#}"));
            } else {
                self.report_error(error);
            }
        }
    }

    pub(super) fn needs_poll(&self) -> bool {
        self.ui.loading
            || self.ui.playing
            || self.pending_seek.is_some()
            || self.settings_due.is_some()
    }

    pub(super) fn has_queue(&self) -> bool {
        !self.queue.is_empty()
    }

    pub(super) fn media_browser_options(&self) -> BrowserOptions {
        self.media_browser_options
    }

    pub(super) fn set_media_browser_options(&mut self, options: BrowserOptions) {
        if self.media_browser_options == options {
            return;
        }
        self.media_browser_options = options;
        match serde_json::to_string(&options) {
            Ok(value) => {
                self.preferences
                    .set(preferences::MEDIA_BROWSER_OPTIONS, Some(value));
                self.schedule_settings();
            }
            Err(error) => eprintln!("OpenPlayer media browser options: {error:#}"),
        }
    }

    pub(super) fn media_browser_directory(&self) -> Option<PathBuf> {
        if self.ui.incognito {
            return None;
        }
        if let Some(directory) = &self.last_media_directory {
            return Some(directory.clone());
        }
        Path::new(&self.history.first()?.path)
            .parent()
            .map(Path::to_path_buf)
    }

    pub(super) fn remember_media_directory(&mut self, directory: &Path) {
        if self.ui.incognito || self.last_media_directory.as_deref() == Some(directory) {
            return;
        }
        if let Some(directory_text) = directory.to_str() {
            self.last_media_directory = Some(directory.to_path_buf());
            self.preferences.set(
                preferences::LAST_MEDIA_DIRECTORY,
                Some(directory_text.to_owned()),
            );
            self.schedule_settings();
        }
    }

    pub(super) fn favorite_directories(&self) -> &[PathBuf] {
        &self.favorite_directories
    }

    pub(super) fn favorite_media(&self) -> &[PathBuf] {
        &self.favorite_media
    }

    pub(super) fn is_media_favorite(&self, path: &Path) -> bool {
        self.favorite_media
            .iter()
            .any(|favorite| media::same_local_path(favorite, path))
    }

    pub(super) fn toggle_favorite_directory(&mut self, directory: PathBuf) {
        self.toggle_browser_favorite(directory, false);
    }

    pub(super) fn toggle_favorite_media(&mut self, path: PathBuf) {
        if crate::media_paths::is_supported_media_path(&path) {
            self.toggle_browser_favorite(path, true);
        }
    }

    fn toggle_browser_favorite(&mut self, path: PathBuf, is_media: bool) {
        if !path.is_absolute() || path.to_str().is_none() {
            return;
        }
        let (favorites, key) = if is_media {
            (&mut self.favorite_media, preferences::FAVORITE_MEDIA)
        } else {
            (
                &mut self.favorite_directories,
                preferences::FAVORITE_DIRECTORIES,
            )
        };
        if let Some(index) = favorites
            .iter()
            .position(|favorite| media::same_local_path(favorite, &path))
        {
            favorites.remove(index);
        } else {
            favorites.push(path);
        }
        match serde_json::to_string(favorites) {
            Ok(value) => {
                self.preferences.set(key, Some(value));
                self.schedule_settings();
            }
            Err(error) => eprintln!("OpenPlayer media favorites: {error:#}"),
        }
    }

    pub(super) fn active_media_info(
        &self,
    ) -> Option<(String, Option<PathBuf>, super::media_info::MediaInfo)> {
        let current = self.current.as_ref().filter(|_| self.ui.loaded)?;
        let mpv = self.mpv.as_ref()?;
        let location = (!current.network).then(|| PathBuf::from(&current.source));
        Some((
            current.name.clone(),
            location,
            super::media_info::snapshot(mpv),
        ))
    }

    fn restore_settings(&mut self) -> Result<()> {
        self.restore_preferences();
        self.version();
        self.load_local_labels();
        if self.store_path.is_file() {
            let store = self.store()?;
            self.playback = store.settings().map_err(anyhow::Error::msg)?;
            self.set_history(store.list().map_err(anyhow::Error::msg)?);
            self.stream_history =
                Arc::new(store.network_stream_history().map_err(anyhow::Error::msg)?);
        }
        self.ui.volume = self.playback.volume / 100.;
        self.ui.speed = self.playback.playback_speed;
        self.ui.speed_label = format!("{}x", self.ui.speed);
        self.ui.loop_mode.clone_from(&self.playback.loop_mode);
        self.ui.video_fill = self.playback.video_fill;
        self.ui.time_display_frames = self.playback.time_display_mode == "frames";
        self.ui.software_decode = self.playback.hwdec_mode == "software";
        Ok(())
    }

    fn hardware_decode(&self) -> &'static str {
        if self.playback.hwdec_mode == "software" {
            "no"
        } else {
            "auto-safe"
        }
    }

    fn apply_video_layout(&self, mpv: &Mpv) -> Result<()> {
        mpv.set_property("panscan", if self.playback.video_fill { 1.0 } else { 0.0 })?;
        Ok(())
    }

    fn video_aspect_ratio(mpv: &Mpv) -> Option<f64> {
        let ratio = mpv
            .get_property::<f64>("video-out-params/aspect")
            .ok()
            .filter(|ratio| ratio.is_finite() && *ratio > 0.)?;
        let rotation = mpv
            .get_property::<i64>("video-out-params/rotate")
            .unwrap_or(0);
        Some(if rotation.rem_euclid(180) == 90 {
            ratio.recip()
        } else {
            ratio
        })
    }

    fn refresh(&mut self, now: Instant) -> Result<()> {
        if let Some(mpv) = self.mpv.clone() {
            self.refresh_playback(&mpv, now)?;
        }
        if self.settings_due.is_some_and(|due| now >= due) {
            self.flush_settings();
        }
        Ok(())
    }

    fn refresh_playback(&mut self, mpv: &Mpv, now: Instant) -> Result<()> {
        let mut tracks_changed = false;
        let mut opened = false;
        let mut ended = false;
        for _ in 0..48 {
            match mpv.wait_event(0.) {
                Some(Ok(Event::FileLoaded)) => opened = true,
                Some(Ok(Event::VideoReconfig | Event::AudioReconfig)) => tracks_changed = true,
                Some(Ok(Event::EndFile(reason)))
                    if reason == mpv_end_file_reason_MPV_END_FILE_REASON_EOF =>
                {
                    ended = true;
                }
                Some(Err(error)) if self.ui.loading => {
                    let path = mpv.get_property::<String>("path").ok();
                    if path.is_none_or(|path| {
                        self.current
                            .as_ref()
                            .is_some_and(|target| target.source == path)
                    }) {
                        self.clear_media();
                        self.report_error(anyhow::Error::new(error));
                    }
                }
                Some(_) => {}
                None => break,
            }
        }
        if let Some(target) = self.current.as_ref() {
            if opened
                && (target.network
                    || mpv.get_property::<String>("path").is_ok_and(|path| {
                        media::same_local_path(Path::new(&path), Path::new(&target.source))
                    }))
            {
                if !target.network {
                    self.sidecar_tracks =
                        media::load_sidecar_subtitles(mpv, Path::new(&target.source));
                    self.select_initial_subtitle(mpv);
                }
                self.ui.loading = false;
                self.ui.loaded = true;
                tracks_changed = true;
            }
            if self.ui.loaded {
                let eof = ended || mpv.get_property::<bool>("eof-reached").unwrap_or(false);
                self.sample_clock(mpv, now, eof);
                if tracks_changed {
                    self.refresh_tracks(mpv);
                }
                if self.ui.audio_only {
                    self.ui.video_aspect_ratio = None;
                } else if tracks_changed || self.ui.video_aspect_ratio.is_none() {
                    self.ui.video_aspect_ratio = Self::video_aspect_ratio(mpv);
                }
                if !self.ui.incognito && now.duration_since(self.last_history) >= HISTORY_INTERVAL {
                    self.persist_history();
                }
                if eof && !self.eof_handled {
                    self.eof_handled = true;
                    self.persist_history();
                    match self.playback.loop_mode.as_str() {
                        "one" => self.restart()?,
                        "all" => {
                            if let Some(index) = self.adjacent(1) {
                                self.open_index(index, false)?;
                            } else {
                                self.restart()?;
                            }
                        }
                        _ => {}
                    }
                } else if !eof {
                    self.eof_handled = false;
                }
            }
        }
        Ok(())
    }

    fn sample_clock(&mut self, mpv: &Mpv, now: Instant, eof: bool) {
        let paused = mpv.get_property::<bool>("pause").unwrap_or(true);
        let buffering = mpv
            .get_property::<bool>("paused-for-cache")
            .unwrap_or(false);
        let seeking = mpv.get_property::<bool>("seeking").unwrap_or(false);
        let sample = mpv
            .get_property::<f64>("time-pos")
            .ok()
            .filter(|time| time.is_finite());
        self.ui.duration = mpv
            .get_property::<f64>("duration")
            .ok()
            .filter(|duration| duration.is_finite())
            .unwrap_or(0.)
            .max(0.);
        self.ui.playing = !paused && !eof;
        self.ui.advancing = self.ui.playing && !buffering && !seeking;
        if let Some(pending) = &self.pending_seek {
            let settled = sample.is_some_and(|position| (position - pending.position).abs() < 0.2);
            if settled || now.duration_since(pending.requested) >= SEEK_SETTLE || eof {
                self.pending_seek = None;
            }
        }
        self.ui.position = self
            .pending_seek
            .as_ref()
            .map(|seek| seek.position)
            .or(sample)
            .unwrap_or(self.ui.position)
            .max(0.);
        self.ui.sampled_at = Some(now);
    }

    fn refresh_tracks(&mut self, mpv: &Mpv) {
        let optional =
            |prefix: &str, key: &str| mpv.get_property::<String>(&format!("{prefix}/{key}")).ok();
        let count = media::track_count(mpv);
        self.ui.tracks = (0..count)
            .filter_map(|index| {
                let key = format!("track-list/{index}");
                let id = mpv
                    .get_property::<i64>(&format!("{key}/id"))
                    .ok()?
                    .try_into()
                    .ok()?;
                Some(Track {
                    id,
                    kind: optional(&key, "type")?,
                    title: optional(&key, "title"),
                    language: optional(&key, "lang"),
                    codec: optional(&key, "codec"),
                    selected: mpv
                        .get_property::<bool>(&format!("{key}/selected"))
                        .unwrap_or(false),
                })
            })
            .collect();
        if !self.sidecar_tracks.is_empty() {
            self.ui.tracks.sort_by_key(|track| {
                if track.kind == "sub" {
                    self.sidecar_tracks
                        .iter()
                        .position(|&id| id == i64::from(track.id))
                } else {
                    None
                }
            });
        }
        let audio = self
            .ui
            .tracks
            .iter()
            .find(|track| track.kind == "audio" && track.selected);
        self.ui.audio_codec = audio.and_then(|track| track.codec.clone());
        self.ui.audio_only = audio.is_some()
            && !self
                .ui
                .tracks
                .iter()
                .any(|track| track.kind == "video" && track.selected);
        self.ui.fps = mpv
            .get_property::<f64>("container-fps")
            .or_else(|_| mpv.get_property::<f64>("estimated-vf-fps"))
            .ok()
            .filter(|fps| fps.is_finite() && *fps > 0.);
        self.ui.subtitle_delay = mpv.get_property::<f64>("sub-delay").unwrap_or(0.);
    }

    fn load(&mut self, media: Media, resume: bool) -> Result<()> {
        self.persist_history();
        let resume = if resume && !media.network && !self.ui.incognito && self.store_path.is_file()
        {
            self.store()?
                .resume_position(&media.source)
                .map_err(anyhow::Error::msg)?
        } else {
            0.
        };
        if let Some(mpv) = &self.mpv {
            self.load_file(mpv, &media, resume)?;
        }
        self.clear_media();
        if self.mpv.is_none() {
            self.pending_resume = Some(resume);
        }
        self.ui.media = Some(media.item());
        self.ui.loading = true;
        self.current = Some(media);
        self.last_history = Instant::now();
        self.ui.error = None;
        Ok(())
    }

    fn load_file(&self, mpv: &Mpv, media: &Media, resume: f64) -> Result<()> {
        mpv.set_property("pause", false)?;
        mpv.set_property("volume", self.playback.volume)?;
        media::loadfile(mpv, media, resume)?;
        Ok(())
    }

    fn clear_media(&mut self) {
        self.current = None;
        self.pending_resume = None;
        self.ui.media = None;
        self.ui.loaded = false;
        self.ui.loading = false;
        self.ui.playing = false;
        self.ui.advancing = false;
        self.ui.position = 0.;
        self.ui.duration = 0.;
        self.ui.audio_only = false;
        self.ui.audio_codec = None;
        self.ui.video_aspect_ratio = None;
        self.ui.subtitle_delay = 0.;
        self.ui.tracks.clear();
        self.sidecar_tracks.clear();
        self.ui.fps = None;
        self.ui.sampled_at = None;
        self.pending_seek = None;
        self.eof_handled = false;
        self.reanchor_timeline();
    }

    fn reanchor_timeline(&mut self) {
        self.ui.timeline_generation = self.ui.timeline_generation.wrapping_add(1);
    }

    fn open_paths(&mut self, input: Vec<String>, append: bool) -> Result<()> {
        if input.is_empty() {
            return Ok(());
        }
        let paths = collect_media_files_from_paths(&input).map_err(anyhow::Error::msg)?;
        let Some(first) = paths.first() else {
            return Ok(());
        };
        let directory = local_media_directory(first);
        if !append {
            self.queue = paths.into_iter().map(Media::local).collect();
            self.ui.playlist = self.queue.len() > 1;
            self.open_index(0, true)?;
        } else {
            let previous_len = self.queue.len();
            for path in paths {
                if !self
                    .queue
                    .iter()
                    .any(|entry| entry.source.eq_ignore_ascii_case(&path))
                {
                    self.queue.push(Media::local(path));
                }
            }
            if self.queue.len() != previous_len {
                self.ui.playlist = self.queue.len() > 1;
            }
            self.refresh_queue();
            if self.current.is_none() {
                self.open_index(0, true)?;
            }
        }
        if let Some(directory) = directory {
            self.remember_media_directory(&directory);
        }
        Ok(())
    }

    fn open_index(&mut self, index: usize, resume: bool) -> Result<()> {
        if let Some(media) = self.queue.get(index).cloned() {
            self.load(media, resume)?;
            self.ui.queue_index = Some(index);
            self.refresh_queue();
        }
        Ok(())
    }

    fn adjacent(&self, offset: isize) -> Option<usize> {
        let len = self.queue.len();
        if len < 2 {
            return None;
        }
        let index = self.ui.queue_index? as isize + offset;
        if self.playback.loop_mode == "all" {
            return Some(index.rem_euclid(len as isize) as usize);
        }
        (0..len as isize).contains(&index).then_some(index as usize)
    }

    fn refresh_queue(&mut self) {
        self.ui.queue = self.queue.iter().map(Media::item).collect();
        self.ui.queued_previous = self.adjacent(-1).is_some();
        self.ui.queued_next = self.adjacent(1).is_some();
    }

    fn open_stream(&mut self, address: &str) -> Result<()> {
        let (source, name) = network_stream(address).map_err(anyhow::Error::msg)?;
        let media = Media {
            source: source.clone(),
            name: name.clone(),
            network: true,
        };
        self.load(media.clone(), false)?;
        self.queue = vec![media];
        self.ui.queue_index = Some(0);
        self.refresh_queue();
        self.ui.stream = None;
        if !self.ui.incognito {
            self.stream_history = Arc::new(
                self.store()?
                    .remember_network_stream(NetworkStreamHistoryUpdate {
                        url: source,
                        name: Some(name),
                        updated_at: None,
                    })
                    .map_err(anyhow::Error::msg)?,
            );
        }
        Ok(())
    }

    fn select_initial_subtitle(&self, mpv: &Mpv) {
        let saved = self
            .saved_subtitle_track()
            .filter(|id| id.is_none_or(|id| media::has_subtitle_track(mpv, id)));
        if let Some(track) = saved.or_else(|| self.sidecar_tracks.first().copied().map(Some)) {
            let _ = Self::apply_track(mpv, "sid", track);
        }
    }

    fn saved_subtitle_track(&self) -> Option<Option<i64>> {
        let media = self
            .current
            .as_ref()
            .filter(|media| !media.network && !self.ui.incognito)?;
        if !self.store_path.is_file() {
            return None;
        }
        let saved = self.store().ok()?.media_settings(&media.source).ok()?;
        saved
            .has_subtitle_track_selection
            .then_some(saved.subtitle_track_id)
    }

    fn apply_track(mpv: &Mpv, property: &str, id: Option<i64>) -> Result<()> {
        let selected = id.map_or_else(|| "no".to_owned(), |id| id.to_string());
        mpv.set_property(property, selected)?;
        Ok(())
    }

    fn select_track(&mut self, kind: &str, id: Option<i64>) -> Result<()> {
        let Some(mpv) = self.mpv.clone() else {
            return Ok(());
        };
        let property = match kind {
            "audio" => "aid",
            "video" => "vid",
            "subtitle" => "sid",
            _ => return Ok(()),
        };
        Self::apply_track(&mpv, property, id)?;
        if property == "sid"
            && !self.ui.incognito
            && let Some(media) = self.current.as_ref().filter(|media| !media.network)
        {
            self.store()?
                .update_media_settings(
                    &media.source,
                    MediaPlaybackSettingsUpdate {
                        subtitle_track_id: Some(id),
                    },
                )
                .map_err(anyhow::Error::msg)?;
        }
        self.refresh_tracks(&mpv);
        Ok(())
    }

    fn seek(&mut self, seconds: f64) -> Result<()> {
        let Some(mpv) = &self.mpv else {
            return Ok(());
        };
        if !self.ui.loaded
            || !seconds.is_finite()
            || !mpv.get_property::<bool>("seekable").unwrap_or(false)
        {
            return Ok(());
        }
        let position = if self.ui.duration > 0. {
            seconds.clamp(0., self.ui.duration)
        } else {
            seconds.max(0.)
        };
        mpv.command("seek", &[&position.to_string(), "absolute+exact"])?;
        self.pending_seek = Some(PendingSeek {
            position,
            requested: Instant::now(),
        });
        self.reanchor_timeline();
        Ok(())
    }

    fn restart(&mut self) -> Result<()> {
        if self.ui.loaded
            && let Some(mpv) = self.mpv.clone()
        {
            self.seek(0.)?;
            mpv.set_property("pause", false)?;
        } else if !self.ui.loading {
            self.open_index(self.ui.queue_index.unwrap_or(0), false)?;
        }
        Ok(())
    }

    fn report_error(&mut self, error: anyhow::Error) {
        eprintln!("OpenPlayer: {error:#}");
        self.ui.error = Some(format!("{error:#}"));
    }

    fn store(&self) -> Result<PlaybackStore> {
        PlaybackStore::open(self.store_path.clone()).map_err(anyhow::Error::msg)
    }

    fn persist_history(&mut self) {
        self.last_history = Instant::now();
        let Some(media) = self
            .current
            .as_ref()
            .filter(|media| !media.network && !self.ui.incognito && self.ui.loaded)
        else {
            return;
        };
        let result = self.store().and_then(|mut store| {
            store
                .remember(PlaybackHistoryUpdate {
                    path: media.source.clone(),
                    name: Some(media.name.clone()),
                    position: self.ui.position,
                    duration: self.ui.duration,
                    updated_at: None,
                })
                .map_err(anyhow::Error::msg)
        });
        match result {
            Ok(history) => self.set_history(history),
            Err(error) => eprintln!("OpenPlayer history: {error:#}"),
        }
    }

    fn set_history(&mut self, entries: Vec<PlaybackHistoryEntry>) {
        self.ui.history = entries
            .iter()
            .map(|entry| HistoryEntry {
                name: entry.name.clone(),
                position: entry.position,
                duration: entry.duration,
            })
            .collect();
        self.history = Arc::new(entries);
    }

    fn schedule_settings(&mut self) {
        self.settings_due = Some(Instant::now() + SETTINGS_DELAY);
    }

    fn flush_settings(&mut self) {
        if let Err(error) = self.preferences.flush() {
            eprintln!("OpenPlayer preferences: {error:#}");
        }
        let update = std::mem::take(&mut self.pending_settings);
        if (update.volume.is_some()
            || update.loop_mode.is_some()
            || update.hwdec_mode.is_some()
            || update.playback_speed.is_some()
            || update.video_fill.is_some()
            || update.time_display_mode.is_some())
            && let Err(error) = self.store().and_then(|mut store| {
                store
                    .update_settings(update.clone())
                    .map_err(anyhow::Error::msg)
            })
        {
            self.pending_settings = update;
            eprintln!("OpenPlayer playback settings: {error:#}");
        }
        self.settings_due = None;
    }
}

fn local_media_directory(path: &str) -> Option<PathBuf> {
    std::path::absolute(path)
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.persist_history();
        self.flush_settings();
    }
}
