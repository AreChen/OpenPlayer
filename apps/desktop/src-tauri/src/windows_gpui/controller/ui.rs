use std::{
    collections::BTreeMap,
    iter::once,
    path::{Path, PathBuf},
    ptr,
    sync::Arc,
};

use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use windows_sys::Win32::{UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL};

use super::{
    Controller,
    preferences::{
        ACCENT, DEFAULT_CHORDS, DEFAULT_TITLEBAR_OPACITY, DEFAULT_TRANSPORT_OPACITY,
        FAVORITE_DIRECTORIES, FAVORITE_MEDIA, INCOGNITO, LANGUAGE, LAST_MEDIA_DIRECTORY,
        MEDIA_BROWSER_OPTIONS, QUIET_KEYBOARD, SHORTCUTS, TITLEBAR_OPACITY, TRANSPORT_OPACITY,
        default_chords,
    },
};
use crate::windows_gpui::{
    locale,
    ui_state::{
        ContextMenu, LabeledId, LoopMode, MediaPanel, MenuItem, MenuPosition, NamedItem, Shortcut,
        Speed, StreamDialog, Theme, Version,
    },
};

const REPOSITORY: &str = "https://github.com/AreChen/OpenPlayer";
const RELEASES: &str = "https://github.com/AreChen/OpenPlayer/releases";
const DEFAULT_THEME: &str = "studio-dark";

impl Controller {
    pub(super) fn version(&mut self) {
        self.ui.version = Some(Version {
            name: "OpenPlayer".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "MIT".into(),
            repository: REPOSITORY.into(),
            releases_url: RELEASES.into(),
        });
    }

    pub(super) fn restore_preferences(&mut self) {
        let stored = match self.preferences.read() {
            Ok(stored) => stored,
            Err(error) => {
                eprintln!("OpenPlayer preferences: {error:#}");
                BTreeMap::new()
            }
        };
        let opacity = |key, default| {
            stored
                .get(key)
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|value| value.is_finite())
                .map_or(default, |value| value.clamp(0., 1.))
        };
        self.ui.titlebar_opacity = opacity(TITLEBAR_OPACITY, DEFAULT_TITLEBAR_OPACITY);
        self.ui.transport_opacity = opacity(TRANSPORT_OPACITY, DEFAULT_TRANSPORT_OPACITY);
        self.ui.language = stored
            .get(LANGUAGE)
            .cloned()
            .unwrap_or_else(|| "system".into());
        self.ui.incognito = stored.get(INCOGNITO).is_some_and(|value| value == "true");
        self.ui.quiet_keyboard = stored
            .get(QUIET_KEYBOARD)
            .is_some_and(|value| value == "true");
        self.ui.accent_override = stored.get(ACCENT).and_then(|value| valid_accent(value));
        self.ui.shortcut_bindings = self
            .preferences
            .load_shortcuts(stored.get(SHORTCUTS).map(String::as_str));
        let favorites = |key| {
            let mut paths: Vec<PathBuf> = stored
                .get(key)
                .and_then(|value| serde_json::from_str(value).ok())
                .unwrap_or_default();
            paths.retain(|path| path.is_absolute());
            paths
        };
        self.favorite_directories = favorites(FAVORITE_DIRECTORIES);
        self.favorite_media = favorites(FAVORITE_MEDIA);
        self.favorite_media
            .retain(|path| crate::media_paths::is_supported_media_path(path));
        self.media_browser_options = stored
            .get(MEDIA_BROWSER_OPTIONS)
            .and_then(|value| serde_json::from_str(value).ok())
            .unwrap_or_default();
        self.last_media_directory = stored
            .get(LAST_MEDIA_DIRECTORY)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute());
        self.ui.accents = [
            "#caa05d", "#78d5b3", "#93b4ff", "#d78372", "#b48cf2", "#e4b95f",
        ]
        .map(str::to_owned)
        .to_vec();
        self.ui.theme_id = Some(DEFAULT_THEME.into());
        self.ui.themes = vec![Theme {
            id: DEFAULT_THEME.into(),
            name: "Studio Dark".into(),
            accent: "#caa05d".into(),
        }];
        self.refresh_appearance();
    }

    pub(super) fn load_local_labels(&mut self) {
        let language = self.ui.language.as_str();
        self.ui.labels = Arc::new(locale::labels(language).clone());
        self.ui.loops = ["off", "one", "all"]
            .map(|mode| LoopMode {
                mode: mode.into(),
                label: locale::label(language, &format!("loop.{mode}.label")).into(),
                description: locale::label(language, &format!("loop.{mode}.description")).into(),
            })
            .to_vec();
        self.ui.languages = ["system", "en-US", "zh-CN"]
            .map(|id| LabeledId {
                id: id.into(),
                label: locale::label(language, &format!("settings.appearance.languages.{id}"))
                    .into(),
            })
            .to_vec();
        self.ui.speeds = [0.5, 0.75, 1., 1.25, 1.5, 2.]
            .map(|value| Speed {
                value,
                label: format!("{value}x"),
            })
            .to_vec();
        self.refresh_shortcuts();
    }

    fn refresh_shortcuts(&mut self) {
        self.ui.shortcut_rows = DEFAULT_CHORDS
            .iter()
            .map(|(id, _)| Shortcut {
                id: (*id).into(),
                label: locale::label(&self.ui.language, &format!("shortcuts.actions.{id}")).into(),
                binding: self.ui.shortcut_bindings.get(*id).cloned(),
            })
            .collect();
    }

    fn save_shortcuts(&mut self) -> Result<()> {
        self.preferences
            .save_shortcuts(&self.ui.shortcut_bindings)?;
        self.refresh_shortcuts();
        self.schedule_settings();
        Ok(())
    }

    fn refresh_appearance(&mut self) {
        self.ui.appearance = self.ui.accent_override.as_ref().map(|accent| {
            [("--accent".to_string(), accent.clone())]
                .into_iter()
                .collect()
        });
    }

    fn set_accent(&mut self, color: Option<&str>) -> Result<()> {
        let color = color
            .map(|text| {
                valid_accent(text).ok_or_else(|| anyhow!("Expected a hex color such as #caa05d"))
            })
            .transpose()?;
        self.ui.accent_override.clone_from(&color);
        self.preferences.set(ACCENT, color);
        self.refresh_appearance();
        self.schedule_settings();
        Ok(())
    }

    fn set_opacity(&mut self, key: &'static str, opacity: f64) {
        let opacity = opacity.clamp(0., 1.);
        if key == TITLEBAR_OPACITY {
            self.ui.titlebar_opacity = opacity;
        } else {
            self.ui.transport_opacity = opacity;
        }
        self.preferences.set(key, Some(opacity.to_string()));
        self.schedule_settings();
    }

    pub(super) fn stream_error(&mut self, error: String) {
        if let Some(stream) = self.ui.stream.as_mut() {
            stream.error = Some(error);
        } else {
            self.ui.error = Some(error);
        }
    }

    fn show_stream(&mut self) {
        self.ui.stream = Some(StreamDialog {
            error: None,
            history: self
                .stream_history
                .iter()
                .map(|entry| NamedItem {
                    name: entry.name.clone(),
                })
                .collect(),
        });
    }

    fn context_menu(&mut self, position: MenuPosition) {
        struct Row<'a> {
            action: &'a str,
            label: &'a str,
            icon: &'a str,
            shortcut: Option<&'a str>,
            enabled: bool,
        }

        let loaded = self.ui.loaded;
        let fit_window = self.ui.video_aspect_ratio.map(|_| Row {
            action: "fitWindowToVideo",
            label: "fitWindowToVideo",
            icon: "restore",
            shortcut: Some("fitWindowToVideo"),
            enabled: true,
        });
        let mut items = Vec::with_capacity(18);
        let groups = [
            vec![
                Row {
                    action: "openMedia",
                    label: "openMedia",
                    icon: "folder",
                    shortcut: Some("openMedia"),
                    enabled: true,
                },
                Row {
                    action: "appendFiles",
                    label: "appendFiles",
                    icon: "folderAdd",
                    shortcut: None,
                    enabled: true,
                },
                Row {
                    action: "appendFolder",
                    label: "appendFolder",
                    icon: "folderAdd",
                    shortcut: None,
                    enabled: true,
                },
                Row {
                    action: "openNetworkStream",
                    label: "openNetworkStream",
                    icon: "stream",
                    shortcut: None,
                    enabled: true,
                },
            ],
            vec![
                Row {
                    action: "togglePlayback",
                    label: if self.ui.playing { "pause" } else { "play" },
                    icon: if self.ui.playing { "pause" } else { "play" },
                    shortcut: Some("togglePlayback"),
                    enabled: true,
                },
                Row {
                    action: "stopPlayback",
                    label: "stop",
                    icon: "stop",
                    shortcut: None,
                    enabled: loaded,
                },
                Row {
                    action: "restartPlayback",
                    label: "restart",
                    icon: "restart",
                    shortcut: Some("restart"),
                    enabled: loaded,
                },
                Row {
                    action: "toggleTracks",
                    label: "tracksSubtitles",
                    icon: "mediaTracks",
                    shortcut: None,
                    enabled: true,
                },
                Row {
                    action: "togglePlaylist",
                    label: "playlist",
                    icon: "list",
                    shortcut: Some("togglePlaylist"),
                    enabled: true,
                },
                Row {
                    action: "openFileLocation",
                    label: "openFileLocation",
                    icon: "folder",
                    shortcut: None,
                    enabled: self.current.as_ref().is_some_and(|media| !media.network),
                },
                Row {
                    action: "mediaProperties",
                    label: "mediaProperties",
                    icon: "info",
                    shortcut: None,
                    enabled: loaded,
                },
            ]
            .into_iter()
            .chain(fit_window)
            .collect(),
            vec![
                Row {
                    action: "toggleFullscreen",
                    label: "fullscreen",
                    icon: "fullscreen",
                    shortcut: Some("toggleFullscreen"),
                    enabled: true,
                },
                Row {
                    action: "toggleAlwaysOnTop",
                    label: "alwaysOnTop",
                    icon: "pin",
                    shortcut: Some("toggleAlwaysOnTop"),
                    enabled: true,
                },
                Row {
                    action: "openSettings",
                    label: "settings",
                    icon: "settings",
                    shortcut: Some("openSettings"),
                    enabled: true,
                },
                Row {
                    action: "closeWindow",
                    label: "closeWindow",
                    icon: "close",
                    shortcut: None,
                    enabled: true,
                },
            ],
        ];
        for (index, group) in groups.into_iter().enumerate() {
            if index != 0 {
                items.push(MenuItem {
                    id: format!("divider-{index}"),
                    label: None,
                    icon: None,
                    shortcut: None,
                    disabled: false,
                    divider: true,
                });
            }
            items.extend(group.into_iter().map(|row| {
                MenuItem {
                    id: row.action.into(),
                    label: Some(
                        locale::label(&self.ui.language, &format!("contextMenu.{}", row.label))
                            .into(),
                    ),
                    icon: Some(row.icon.into()),
                    shortcut: row
                        .shortcut
                        .and_then(|action| self.ui.shortcut_bindings.get(action).cloned()),
                    disabled: !row.enabled,
                    divider: false,
                }
            }));
        }
        self.ui.context = Some(ContextMenu { position, items });
    }

    fn toggle_compact_context_menu(&mut self) {
        if self
            .ui
            .context
            .as_ref()
            .is_some_and(|menu| matches!(menu.position, MenuPosition::PlaybackControls))
        {
            self.ui.context = None;
            return;
        }

        let (decode_label, decode_icon) = if self.ui.software_decode {
            ("hardware.switchToHardware", "decodeHardware")
        } else {
            ("hardware.switchToSoftware", "decodeSoftware")
        };
        let playlist_label = if self.ui.playlist {
            "controls.hidePlaylist"
        } else {
            "controls.showPlaylist"
        };
        let items = [
            ("stopPlayback", "controls.stop", "stop", self.ui.loaded),
            (
                "previous",
                "controls.previousVideo",
                "previous",
                self.ui.queued_previous,
            ),
            ("next", "controls.nextVideo", "next", self.ui.queued_next),
            ("toggleLoop", "controls.openLoopMode", "repeat", true),
            ("toggleSpeed", "controls.openPlaybackSpeed", "speed", true),
            ("toggleDecode", decode_label, decode_icon, true),
            ("toggleTracks", "controls.openTracks", "subtitles", true),
            ("togglePlaylist", playlist_label, "list", true),
        ]
        .into_iter()
        .map(|(action, label, icon, enabled)| MenuItem {
            id: action.into(),
            label: Some(locale::label(&self.ui.language, label).into()),
            icon: Some(icon.into()),
            shortcut: self.ui.shortcut_bindings.get(action).cloned(),
            disabled: !enabled,
            divider: false,
        })
        .collect();

        self.ui.context = Some(ContextMenu {
            position: MenuPosition::PlaybackControls,
            items,
        });
    }

    pub(super) fn action(&mut self, kind: &str, value: Value) -> Result<()> {
        match kind {
            "paths" | "appendPaths" => {
                let paths = value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                self.open_paths(paths, kind == "appendPaths")?;
            }
            "togglePlayback" => {
                if self.ui.loaded
                    && !self.eof_handled
                    && let Some(mpv) = &self.mpv
                {
                    let paused = mpv.get_property::<bool>("pause").unwrap_or(true);
                    mpv.set_property("pause", !paused)?;
                    self.reanchor_timeline();
                } else if self.eof_handled {
                    self.restart()?;
                } else if !self.ui.loading {
                    self.open_index(self.ui.queue_index.unwrap_or(0), true)?;
                }
            }
            "stopPlayback" => {
                self.persist_history();
                if let Some(mpv) = &self.mpv {
                    mpv.command("stop", &[])?;
                }
                self.clear_media();
            }
            "restartPlayback" => self.restart()?,
            "previous" | "next" => {
                if let Some(index) = self.adjacent(if kind == "next" { 1 } else { -1 }) {
                    self.open_index(index, true)?;
                }
            }
            "queueItem" => {
                if let Some(index) = value.as_u64().and_then(|index| index.try_into().ok()) {
                    self.open_index(index, true)?;
                    if let Some(directory) = self
                        .queue
                        .get(index)
                        .filter(|media| !media.network)
                        .and_then(|media| super::local_media_directory(&media.source))
                    {
                        self.remember_media_directory(&directory);
                    }
                }
            }
            "historyItem" => {
                if let Some(entry) = value
                    .as_u64()
                    .and_then(|index| self.history.get(index as usize))
                {
                    self.open_paths(vec![entry.path.clone()], false)?;
                }
            }
            "clearHistory" => {
                let history = self.store()?.clear().map_err(anyhow::Error::msg)?;
                self.set_history(history);
            }
            "seek" | "seekBy" => {
                if let Some(value) = value.as_f64() {
                    let seconds = if kind == "seekBy" {
                        self.ui.position + value
                    } else {
                        value
                    };
                    self.seek(seconds)?;
                }
            }
            "frameBackward" | "frameForward" => {
                if self.ui.loaded
                    && !self.ui.audio_only
                    && let Some(mpv) = &self.mpv
                {
                    mpv.set_property("pause", true)?;
                    let command = if kind == "frameForward" {
                        "frame-step"
                    } else {
                        "frame-back-step"
                    };
                    mpv.command(command, &[])?;
                    self.pending_seek = None;
                    self.reanchor_timeline();
                }
            }
            "toggleTime" => {
                if self.ui.fps.is_some() && self.ui.duration > 0. {
                    self.ui.time_display_frames = !self.ui.time_display_frames;
                    self.playback.time_display_mode = if self.ui.time_display_frames {
                        "frames"
                    } else {
                        "timecode"
                    }
                    .into();
                    self.pending_settings.time_display_mode =
                        Some(self.playback.time_display_mode.clone());
                    self.schedule_settings();
                }
            }
            "toggleMute" => {
                if let Some(mpv) = &self.mpv {
                    mpv.set_property("mute", !self.ui.muted)?;
                }
                self.ui.muted = !self.ui.muted;
            }
            "volume" => {
                if let Some(value) = value.as_f64() {
                    let volume = value.clamp(0., 1.);
                    let unmute = self.ui.muted && volume > 0.;
                    if let Some(mpv) = &self.mpv {
                        mpv.set_property("volume", volume * 100.)?;
                        if unmute {
                            mpv.set_property("mute", false)?;
                        }
                    }
                    if unmute {
                        self.ui.muted = false;
                    }
                    self.ui.volume = volume;
                    self.playback.volume = volume * 100.;
                    self.pending_settings.volume = Some(self.playback.volume);
                    self.schedule_settings();
                }
            }
            "toggleDecode" => {
                let software = self.playback.hwdec_mode == "software";
                if let Some(mpv) = &self.mpv {
                    mpv.set_property("hwdec", if software { "auto-safe" } else { "no" })?;
                }
                self.playback.hwdec_mode = if software { "hardware" } else { "software" }.into();
                self.pending_settings.hwdec_mode = Some(self.playback.hwdec_mode.clone());
                self.ui.software_decode = !software;
                self.schedule_settings();
            }
            "speed" => {
                if let Some(value) = value.as_f64() {
                    let speed = value.clamp(0.25, 4.);
                    if let Some(mpv) = &self.mpv {
                        mpv.set_property("speed", speed)?;
                    }
                    self.ui.speed = speed;
                    self.ui.speed_label = format!("{speed}x");
                    self.playback.playback_speed = speed;
                    self.pending_settings.playback_speed = Some(speed);
                    self.reanchor_timeline();
                    self.schedule_settings();
                }
            }
            "loop" => {
                if let Some(mode @ ("off" | "one" | "all")) = value.as_str() {
                    self.playback.loop_mode = mode.into();
                    self.ui.loop_mode = mode.into();
                    self.pending_settings.loop_mode = Some(self.playback.loop_mode.clone());
                    self.refresh_queue();
                    self.schedule_settings();
                }
            }
            "videoFill" => {
                if let Some(fill) = value.as_bool() {
                    self.playback.video_fill = fill;
                    if let Some(mpv) = &self.mpv {
                        self.apply_video_layout(mpv)?;
                    }
                    self.ui.video_fill = fill;
                    self.pending_settings.video_fill = Some(fill);
                    self.schedule_settings();
                }
            }
            "subtitleDelay" => {
                if let Some(delay) = value.as_f64()
                    && self.ui.loaded
                    && let Some(mpv) = &self.mpv
                {
                    let delay = delay.clamp(-10., 10.);
                    mpv.set_property("sub-delay", delay)?;
                    self.ui.subtitle_delay = delay;
                }
            }
            "addSubtitlePath" => {
                if let Some(path) = value.as_str().filter(|path| Path::new(path).is_file())
                    && self.ui.loaded
                    && let Some(mpv) = self.mpv.clone()
                {
                    mpv.command("sub-add", &[path, "select"])?;
                    self.refresh_tracks(&mpv);
                }
            }
            "track" => {
                if let Some(track) = value.as_array()
                    && let Some(kind) = track.first().and_then(Value::as_str)
                {
                    self.select_track(kind, track.get(1).and_then(Value::as_i64))?;
                }
            }
            "toggleSpeed" => self.toggle_panel(MediaPanel::Speed),
            "toggleTracks" => self.toggle_panel(MediaPanel::Tracks),
            "toggleLoop" => self.toggle_panel(MediaPanel::Loop),
            "togglePlaylist" => self.ui.playlist = !self.ui.playlist,
            "closePanels" => {
                self.ui.playlist = false;
                self.ui.panel = None;
            }
            "openSettings" => self.ui.settings = Some("appearance".into()),
            "closeSettings" => {
                self.ui.settings = None;
                self.ui.recording_shortcut = None;
            }
            "settingsSection" => {
                self.ui.settings = value.as_str().map(str::to_owned);
                self.ui.recording_shortcut = None;
            }
            "theme" => self.refresh_appearance(),
            "accent" => self.set_accent(value.as_str())?,
            "titlebarOpacity" | "transportOpacity" => {
                if let Some(opacity) = value.as_f64() {
                    let key = if kind == TITLEBAR_OPACITY {
                        TITLEBAR_OPACITY
                    } else {
                        TRANSPORT_OPACITY
                    };
                    self.set_opacity(key, opacity);
                }
            }
            "language" => {
                if let Some(language @ ("system" | "en-US" | "zh-CN")) = value.as_str() {
                    self.ui.language = language.into();
                    self.preferences.set(LANGUAGE, Some(language.into()));
                    self.load_local_labels();
                    self.schedule_settings();
                }
            }
            "resetAppearance" => {
                self.set_accent(None)?;
                self.set_opacity(TITLEBAR_OPACITY, DEFAULT_TITLEBAR_OPACITY);
                self.set_opacity(TRANSPORT_OPACITY, DEFAULT_TRANSPORT_OPACITY);
            }
            "incognito" | "quietKeyboard" => {
                if let Some(enabled) = value.as_bool() {
                    let key = if kind == "incognito" {
                        self.ui.incognito = enabled;
                        INCOGNITO
                    } else {
                        self.ui.quiet_keyboard = enabled;
                        QUIET_KEYBOARD
                    };
                    self.preferences.set(key, Some(enabled.to_string()));
                    self.schedule_settings();
                }
            }
            "recordShortcut" => self.ui.recording_shortcut = value.as_str().map(str::to_owned),
            "assignShortcut" => {
                if let Some(binding) = value.as_array()
                    && let Some(action) = binding.first().and_then(Value::as_str)
                    && DEFAULT_CHORDS.iter().any(|(id, _)| *id == action)
                {
                    self.ui.shortcut_bindings.remove(action);
                    if let Some(chord) = binding.get(1).and_then(Value::as_str) {
                        self.ui
                            .shortcut_bindings
                            .retain(|_, current| current != chord);
                        self.ui
                            .shortcut_bindings
                            .insert(action.into(), chord.into());
                    }
                    self.ui.recording_shortcut = None;
                    self.save_shortcuts()?;
                }
            }
            "resetShortcuts" => {
                self.ui.recording_shortcut = None;
                self.ui.shortcut_bindings = default_chords();
                self.save_shortcuts()?;
            }
            "openNetworkStream" => self.show_stream(),
            "closeNetworkStream" => self.ui.stream = None,
            "submitStream" => self.open_stream(value.as_str().unwrap_or_default())?,
            "clearNetworkHistory" => {
                self.stream_history = Arc::new(
                    self.store()?
                        .clear_network_stream_history()
                        .map_err(anyhow::Error::msg)?,
                );
                self.show_stream();
            }
            "streamHistoryItem" => {
                if let Some(entry) = value
                    .as_u64()
                    .and_then(|index| self.stream_history.get(index as usize))
                {
                    self.open_stream(&entry.url.clone())?;
                }
            }
            "context" => {
                if let (Some(x), Some(y)) = (value["x"].as_f64(), value["y"].as_f64()) {
                    self.context_menu(MenuPosition::Pointer {
                        x: x as f32,
                        y: y as f32,
                    });
                }
            }
            "compactContext" => self.toggle_compact_context_menu(),
            "closeContext" => self.ui.context = None,
            "contextItem" => {
                self.ui.context = None;
                if let Some(action) = value.as_str() {
                    self.action(action, Value::Null)?;
                }
            }
            "externalUrl" => {
                if let Some(url @ (REPOSITORY | RELEASES)) = value.as_str() {
                    shell_open(url)?;
                }
            }
            "openFileLocation" => {
                if let Some(media) = self.current.as_ref().filter(|media| !media.network) {
                    open_parent_directory(Path::new(&media.source))?;
                }
            }
            "revealBrowserFile" => {
                if let Some(path) = value.as_str() {
                    open_parent_directory(Path::new(path))?;
                }
            }
            "clearError" => self.ui.error = None,
            _ => {}
        }
        Ok(())
    }

    fn toggle_panel(&mut self, panel: MediaPanel) {
        self.ui.panel = if self.ui.panel == Some(panel) {
            None
        } else {
            Some(panel)
        };
        if self.ui.panel == Some(MediaPanel::Tracks)
            && self.ui.loaded
            && let Some(mpv) = self.mpv.clone()
        {
            self.refresh_tracks(&mpv);
        }
    }
}

fn open_parent_directory(media: &Path) -> Result<()> {
    if media.is_absolute()
        && let Some(path) = media.parent().and_then(Path::to_str)
    {
        shell_open(path)?;
    }
    Ok(())
}

fn valid_accent(text: &str) -> Option<String> {
    let text = text.trim();
    let value = text.strip_prefix('#').unwrap_or(text);
    (matches!(value.len(), 3 | 6) && value.bytes().all(|digit| digit.is_ascii_hexdigit())).then(
        || {
            let mut hex = String::from("#");
            for digit in value.chars() {
                hex.extend(
                    if value.len() == 3 {
                        [Some(digit), Some(digit)]
                    } else {
                        [Some(digit), None]
                    }
                    .into_iter()
                    .flatten(),
                );
            }
            hex
        },
    )
}

fn shell_open(target: &str) -> Result<()> {
    let text = target.encode_utf16().chain(once(0)).collect::<Vec<u16>>();
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            ptr::null(),
            text.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as isize <= 32 {
        bail!("Windows could not open {target}");
    }
    Ok(())
}
