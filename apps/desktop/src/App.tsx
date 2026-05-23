import { useEffect, useRef, useState, type CSSProperties, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { languageModeOptions, resolveLocale, translations, type AppStrings, type LanguageMode } from "./i18n";

type MediaItem = {
  id: string;
  name: string;
  path: string;
};

type PlaybackHistoryEntry = {
  path: string;
  name: string;
  position: number;
  duration: number;
  updatedAt: number;
};

type MpvTrack = {
  id: number;
  kind: "audio" | "video" | "sub";
  title: string | null;
  language: string | null;
  codec: string | null;
  selected: boolean;
  external: boolean;
};

type MpvSnapshot = {
  path: string;
  status: string;
  ended: boolean;
  paused: boolean;
  position: number;
  duration: number;
  fps: number;
  speed: number;
  hwdec: string;
  videoFill: boolean;
  subtitleDelay: number;
  volume: number;
  tracks: MpvTrack[];
};

type PlatformSupport = {
  os: string;
  displayServer: string;
  mpvEmbedVideo: boolean;
  nativeShortcutBridge: boolean;
};

type ThemeTokens = {
  surface: string;
  panel: string;
  panelStrong: string;
  text: string;
  muted: string;
  faint: string;
  accent: string;
  danger: string;
  line: string;
  control: string;
  scrollbarThumb: string;
  scrollbarThumbHover: string;
};

type ThemeCatalogItem = {
  id: string;
  name: string;
  version: string;
  source: "builtIn" | "plugin";
  pluginId: string | null;
  enabled: boolean;
  tokens: ThemeTokens;
};

type ThemePluginSummary = {
  id: string;
  name: string;
  version: string;
  description: string | null;
  enabled: boolean;
  themeCount: number;
};

type AppearanceState = {
  activeThemeId: string;
  accentOverride: string | null;
  themes: ThemeCatalogItem[];
  plugins: ThemePluginSummary[];
};

type PlayerPreferences = {
  incognitoMode: boolean;
  quietKeyboardControls: boolean;
  languageMode: LanguageMode;
};

type ShellPreviewRegistrationSummary = {
  registeredCount: number;
  videoCount: number;
  audioCount: number;
  extensions: string[];
};

type ShellPreviewFormatInfo = {
  extension: string;
  mime: string;
  kind: "video" | "audio";
  common: boolean;
};

type PendingSeek = {
  target: number;
  startedAt: number;
};

type PendingWindowDrag = {
  pointerId: number;
  startX: number;
  startY: number;
};

type ManualResizeDrag = {
  pointerId: number;
  direction: ResizeDirection;
  lastX: number;
  lastY: number;
  pendingDeltaX: number;
  pendingDeltaY: number;
  animationFrameId: number | null;
  resizeCommandInFlight: boolean;
  finishing: boolean;
};

type PlaybackClockAnchor = {
  position: number;
  startedAt: number;
  playing: boolean;
  speed: number;
};

type ThemeStyleProperties = CSSProperties & Record<`--${string}`, string>;
type SettingsSection = "appearance" | "plugins" | "playback" | "shortcuts";
type MediaPanelMode = "speed" | "tracks" | "loop";
type LoopMode = "off" | "one" | "all";
type HardwareDecodingMode = "hardware" | "software";
type TimeDisplayMode = "timecode" | "frames";
type SelectableTrackKind = "audio" | "video" | "subtitle";
type VolumeFeedback = {
  level: number;
};

type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";
type ResizeFeedback = {
  direction: ResizeDirection;
  active: boolean;
};

type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_toggle_fullscreen" | "window_close";
type ShortcutAction =
  | "openMedia"
  | "togglePlayback"
  | "restart"
  | "togglePlaylist"
  | "seekBackward"
  | "seekForward"
  | "frameForward"
  | "frameBackward"
  | "volumeDown"
  | "volumeUp"
  | "toggleFullscreen"
  | "openSettings";
type ShortcutBindings = Record<ShortcutAction, string | null>;
type ShortcutDefinition = {
  action: ShortcutAction;
  label: string;
  group: string;
};
type ContextMenuPosition = {
  x: number;
  y: number;
};
type IconName =
  | "close"
  | "cpu"
  | "folder"
  | "folderAdd"
  | "fullscreen"
  | "list"
  | "maximize"
  | "minimize"
  | "next"
  | "palette"
  | "pause"
  | "play"
  | "plugin"
  | "preview"
  | "previous"
  | "restart"
  | "settings"
  | "stop"
  | "tracks"
  | "volume";

const playableExtensions = [
  "3g2",
  "3gp",
  "3gp2",
  "3gpp",
  "aac",
  "ac3",
  "adts",
  "aif",
  "aifc",
  "aiff",
  "alac",
  "amr",
  "ape",
  "asf",
  "au",
  "avi",
  "awb",
  "caf",
  "dff",
  "divx",
  "dsf",
  "dts",
  "dtshd",
  "dv",
  "dvr-ms",
  "eac3",
  "f4v",
  "flac",
  "flv",
  "gsm",
  "h264",
  "h265",
  "hevc",
  "m1v",
  "m2t",
  "m2ts",
  "m2v",
  "m4a",
  "m4b",
  "m4r",
  "m4v",
  "mk3d",
  "mka",
  "mkv",
  "mlp",
  "mov",
  "mp1",
  "mp2",
  "mp3",
  "mp4",
  "mp4v",
  "mpa",
  "mpc",
  "mpe",
  "mpeg",
  "mpg",
  "mpv",
  "mts",
  "mxf",
  "nsv",
  "nut",
  "oga",
  "ogg",
  "ogm",
  "ogv",
  "opus",
  "qt",
  "ra",
  "rm",
  "rmvb",
  "roq",
  "snd",
  "spx",
  "tak",
  "tod",
  "trp",
  "ts",
  "tta",
  "vob",
  "voc",
  "wav",
  "weba",
  "webm",
  "wm",
  "wma",
  "wmv",
  "wv",
  "y4m",
];
const audioOnlyExtensions = [
  "aac",
  "ac3",
  "adts",
  "aif",
  "aifc",
  "aiff",
  "alac",
  "amr",
  "ape",
  "au",
  "awb",
  "caf",
  "dff",
  "dsf",
  "dts",
  "dtshd",
  "eac3",
  "flac",
  "gsm",
  "m4a",
  "m4b",
  "m4r",
  "mka",
  "mlp",
  "mp1",
  "mp2",
  "mp3",
  "mpa",
  "mpc",
  "oga",
  "ogg",
  "opus",
  "ra",
  "snd",
  "spx",
  "tak",
  "tta",
  "voc",
  "wav",
  "weba",
  "wma",
  "wv",
];
const subtitleExtensions = ["ass", "srt", "ssa", "sub", "vtt"];
const themePluginExtensions = ["json"];
const playbackSpeedOptions = [0.5, 0.75, 1, 1.25, 1.5, 2];
const mediaPathCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });
const accentSwatches = ["#caa05d", "#78d5b3", "#93b4ff", "#d78372", "#b48cf2", "#e4b95f"];
const audioVisualizerBarLevels = [
  0.34, 0.56, 0.42, 0.72, 0.5, 0.82, 0.46, 0.66, 0.38, 0.92, 0.58, 0.76, 0.44, 0.68, 0.52, 0.86,
  0.48, 0.62, 0.36, 0.74, 0.54, 0.88, 0.4, 0.7,
];
const DEFAULT_PLAYER_PREFERENCES: PlayerPreferences = {
  incognitoMode: false,
  quietKeyboardControls: false,
  languageMode: "system",
};
const OPENPLAYER_SHORTCUTS_STORAGE_KEY = "openplayer.shortcuts.v3";
const HISTORY_WRITE_INTERVAL_MS = 1500;
const MIN_RESUME_PROGRESS_RATIO = 0.01;
const RESUME_END_PROGRESS_RATIO = 0.95;
const SEEK_CONFIRM_TOLERANCE_SECONDS = 0.75;
const SEEK_SNAPSHOT_SUPPRESS_MS = 1600;
const AUTO_HIDE_CONTROLS_MS = 5000;
const VOLUME_FEEDBACK_MS = 1100;
const STORE_SYNC_INTERVAL_MS = 1600;
const END_OF_MEDIA_SNAP_TOLERANCE_SECONDS = 0.5;
const CONTEXT_MENU_WIDTH = 236;
const CONTEXT_MENU_HEIGHT = 404;
const DEFAULT_SEEK_STEP_SECONDS = 5;
const DEFAULT_VOLUME_STEP = 0.05;
const WINDOW_DRAG_START_DISTANCE_PX = 4;
const SUBTITLE_DELAY_STEP_SECONDS = 0.1;
const TEXT_ENTRY_INPUT_TYPES = new Set(["", "date", "datetime-local", "email", "month", "number", "password", "search", "tel", "text", "time", "url", "week"]);
const resizeRegions: Array<{ className: string; direction: ResizeDirection }> = [
  { className: "resize-region--north", direction: "North" },
  { className: "resize-region--south", direction: "South" },
  { className: "resize-region--east", direction: "East" },
  { className: "resize-region--west", direction: "West" },
  { className: "resize-region--north-east", direction: "NorthEast" },
  { className: "resize-region--north-west", direction: "NorthWest" },
  { className: "resize-region--south-east", direction: "SouthEast" },
  { className: "resize-region--south-west", direction: "SouthWest" },
];
const shortcutActions: ShortcutAction[] = [
  "openMedia",
  "togglePlayback",
  "restart",
  "togglePlaylist",
  "seekBackward",
  "seekForward",
  "frameBackward",
  "frameForward",
  "volumeDown",
  "volumeUp",
  "toggleFullscreen",
  "openSettings",
];
const defaultShortcutBindings: ShortcutBindings = {
  openMedia: "Ctrl+O",
  togglePlayback: "Space",
  restart: "R",
  togglePlaylist: "P",
  seekBackward: "ArrowLeft",
  seekForward: "ArrowRight",
  frameBackward: "D",
  frameForward: "F",
  volumeDown: "ArrowDown",
  volumeUp: "ArrowUp",
  toggleFullscreen: "Enter",
  openSettings: "Ctrl+,",
};
const surface = new URLSearchParams(window.location.search).get("surface");
const openPlayerLogoUrl = new URL("./assets/openplayer-logo.png", import.meta.url).href;
let mediaItemIdCounter = 0;

function nextMediaItemId() {
  mediaItemIdCounter += 1;
  return `path:${mediaItemIdCounter}`;
}

function normalizeShortcutKey(key: string) {
  const aliases: Record<string, string> = {
    " ": "Space",
    Spacebar: "Space",
    Esc: "Escape",
    Left: "ArrowLeft",
    Right: "ArrowRight",
    Up: "ArrowUp",
    Down: "ArrowDown",
    Del: "Delete",
  };
  const normalized = aliases[key] ?? key;
  if (normalized.length === 1 && /[a-z]/i.test(normalized)) {
    return normalized.toUpperCase();
  }

  return normalized;
}

function keyboardEventToChord(event: KeyboardEvent) {
  const key = normalizeShortcutKey(event.key);
  if (["Alt", "Control", "Meta", "Shift"].includes(key)) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.metaKey) {
    parts.push("Meta");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }

  parts.push(key);
  return parts.join("+");
}

function formatShortcutChord(chord: string | null, t: AppStrings) {
  if (!chord) {
    return t.common.unset;
  }

  return chord
    .split("+")
    .map((part) => {
      const labels: Record<string, string> = {
        ArrowDown: "↓",
        ArrowLeft: "←",
        ArrowRight: "→",
        ArrowUp: "↑",
        Escape: "Esc",
        Meta: "Win",
        Space: "Space",
      };
      return labels[part] ?? part;
    })
    .join(" + ");
}

function colorWithAlpha(color: string, alpha: number) {
  const hex = color.trim().replace(/^#/, "");
  if (![3, 6].includes(hex.length) || !/^[\da-f]+$/i.test(hex)) {
    return color;
  }

  const expanded = hex.length === 3 ? hex.split("").map((part) => part + part).join("") : hex;
  const red = Number.parseInt(expanded.slice(0, 2), 16);
  const green = Number.parseInt(expanded.slice(2, 4), 16);
  const blue = Number.parseInt(expanded.slice(4, 6), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function hexColorForPicker(color: string | null | undefined) {
  const value = color?.trim() ?? "";
  return /^#[\da-f]{6}$/i.test(value) ? value : "#caa05d";
}

function browserLanguages() {
  return navigator.languages?.length ? navigator.languages : [navigator.language || "en-US"];
}

function loopModeOptionsFor(t: AppStrings): Array<{ mode: LoopMode; label: string; description: string }> {
  return [
    { mode: "off", ...t.loop.off },
    { mode: "one", ...t.loop.one },
    { mode: "all", ...t.loop.all },
  ];
}

function shortcutDefinitionsFor(t: AppStrings): ShortcutDefinition[] {
  return [
    { action: "openMedia", label: t.shortcuts.actions.openMedia, group: t.shortcuts.groups.file },
    { action: "togglePlayback", label: t.shortcuts.actions.togglePlayback, group: t.shortcuts.groups.playback },
    { action: "restart", label: t.shortcuts.actions.restart, group: t.shortcuts.groups.playback },
    { action: "togglePlaylist", label: t.shortcuts.actions.togglePlaylist, group: t.shortcuts.groups.playback },
    { action: "seekBackward", label: t.shortcuts.actions.seekBackward, group: t.shortcuts.groups.seek },
    { action: "seekForward", label: t.shortcuts.actions.seekForward, group: t.shortcuts.groups.seek },
    { action: "frameBackward", label: t.shortcuts.actions.frameBackward, group: t.shortcuts.groups.frame },
    { action: "frameForward", label: t.shortcuts.actions.frameForward, group: t.shortcuts.groups.frame },
    { action: "volumeDown", label: t.shortcuts.actions.volumeDown, group: t.shortcuts.groups.volume },
    { action: "volumeUp", label: t.shortcuts.actions.volumeUp, group: t.shortcuts.groups.volume },
    { action: "toggleFullscreen", label: t.shortcuts.actions.toggleFullscreen, group: t.shortcuts.groups.window },
    { action: "openSettings", label: t.shortcuts.actions.openSettings, group: t.shortcuts.groups.window },
  ];
}

function activeThemeFromAppearance(appearance: AppearanceState | null) {
  if (!appearance) {
    return null;
  }

  return appearance.themes.find((theme) => theme.id === appearance.activeThemeId && theme.enabled) ?? appearance.themes.find((theme) => theme.enabled) ?? null;
}

function themeStyleVariables(appearance: AppearanceState | null): ThemeStyleProperties | undefined {
  const theme = activeThemeFromAppearance(appearance);
  if (!theme) {
    return undefined;
  }

  const accent = appearance?.accentOverride ?? theme.tokens.accent;
  return {
    "--surface": theme.tokens.surface,
    "--panel": theme.tokens.panel,
    "--panel-strong": theme.tokens.panelStrong,
    "--text": theme.tokens.text,
    "--muted": theme.tokens.muted,
    "--faint": theme.tokens.faint,
    "--accent": accent,
    "--danger": theme.tokens.danger,
    "--line": theme.tokens.line,
    "--control": theme.tokens.control,
    "--scrollbar-thumb": theme.tokens.scrollbarThumb,
    "--scrollbar-thumb-hover": colorWithAlpha(accent, 0.46),
    "--accent-soft": colorWithAlpha(accent, 0.16),
    "--accent-muted": colorWithAlpha(accent, 0.22),
    "--accent-border": colorWithAlpha(accent, 0.42),
    "--accent-ring": colorWithAlpha(accent, 0.82),
  };
}

function readShortcutBindings() {
  try {
    const stored = window.localStorage.getItem(OPENPLAYER_SHORTCUTS_STORAGE_KEY);
    if (!stored) {
      return defaultShortcutBindings;
    }

    const parsed = JSON.parse(stored) as Partial<Record<ShortcutAction, unknown>>;
    const merged: ShortcutBindings = { ...defaultShortcutBindings };
    for (const action of shortcutActions) {
      const value = parsed[action];
      if (typeof value === "string" || value === null) {
        merged[action] = value;
      }
    }

    return merged;
  } catch {
    return defaultShortcutBindings;
  }
}

function isShortcutAction(value: unknown): value is ShortcutAction {
  return typeof value === "string" && shortcutActions.includes(value as ShortcutAction);
}

function isTextEntryShortcutTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) {
    return false;
  }

  const editable = target.closest("textarea, [contenteditable='true'], [role='textbox']");
  if (editable) {
    return true;
  }

  const input = target.closest("input");
  return input instanceof HTMLInputElement && TEXT_ENTRY_INPUT_TYPES.has(input.type);
}

function releaseShortcutFocusTarget(target: EventTarget | null) {
  if (isTextEntryShortcutTarget(target)) {
    return;
  }

  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
}

function focusOverlayWindow() {
  invoke("window_focus_overlay").catch((error: unknown) => {
    console.warn("Overlay focus restore failed", error);
  });
}

function runWindowCommand(command: WindowCommand) {
  invoke(command)
    .then(() => {
      if (command !== "window_close" && command !== "window_minimize") {
        focusOverlayWindow();
      }
    })
    .catch((error: unknown) => {
      console.error(`Window command failed: ${command}`, error);
    });
}

function startMainWindowDrag() {
  invoke("window_start_drag").catch((error: unknown) => {
    console.error("Window drag failed", error);
  });
}

function startNativeMainWindowResize(direction: ResizeDirection) {
  invoke("window_start_resize", { direction }).catch((error: unknown) => {
    console.error(`Window resize failed: ${direction}`, error);
  });
}

function applyManualMainWindowResize(direction: ResizeDirection, deltaX: number, deltaY: number) {
  return invoke("window_apply_resize_delta", { direction, deltaX, deltaY }).catch((error: unknown) => {
    console.error(`Window resize failed: ${direction}`, error);
  });
}

function applyResizeCursor(direction: ResizeDirection | null) {
  return invoke("window_set_resize_cursor", { direction }).catch((error: unknown) => {
    console.warn("Resize cursor update failed", error);
  });
}

function resizeDirectionClassName(direction: ResizeDirection) {
  return direction.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

function formatTimecode(value: number, totalDuration: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return totalDuration > 3600 ? "0:00:00" : "00:00";
  }

  const totalSeconds = Math.floor(value);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (totalDuration > 3600) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${Math.floor(totalSeconds / 60).toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

function resumePositionWithinDuration(position: number, duration: number) {
  if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0 || position <= 0) {
    return 0;
  }

  const clamped = Math.min(position, duration);
  const ratio = clamped / duration;
  if (ratio < MIN_RESUME_PROGRESS_RATIO || ratio >= RESUME_END_PROGRESS_RATIO) {
    return 0;
  }

  return clamped;
}

async function resumePositionForPath(path: string) {
  try {
    const position = await invoke<number>("history_resume_position", { path });
    return Number.isFinite(position) ? Math.max(0, position) : 0;
  } catch (error) {
    console.warn("Failed to resolve playback resume position", error);
    return 0;
  }
}

function formatHistoryProgress(entry: PlaybackHistoryEntry, t: AppStrings) {
  if (!Number.isFinite(entry.duration) || entry.duration <= 0) {
    return t.status.noRecordedProgress;
  }

  const resumePosition = resumePositionWithinDuration(entry.position, entry.duration);
  if (resumePosition <= 0) {
    return t.status.playFromStart;
  }

  return `${formatTimecode(resumePosition, entry.duration)} / ${formatTimecode(entry.duration, entry.duration)}`;
}

function formatFrameCount(value: number, locale: string) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0";
  }

  return Math.floor(value).toLocaleString(locale);
}

function canDisplayFrames(fps: number, duration: number) {
  return Number.isFinite(fps) && fps > 0 && Number.isFinite(duration) && duration > 0;
}

function clampPlaybackSpeed(value: number) {
  if (!Number.isFinite(value)) {
    return 1;
  }

  return Math.min(4, Math.max(0.25, value));
}

function formatPlaybackSpeed(value: number) {
  const speed = clampPlaybackSpeed(value);
  return `${Number.isInteger(speed) ? speed.toFixed(0) : speed.toFixed(2).replace(/0$/, "")}x`;
}

function loopModeLabel(mode: LoopMode, t: AppStrings) {
  return loopModeOptionsFor(t).find((option) => option.mode === mode)?.label ?? t.loop.off.label;
}

function clampSubtitleDelay(value: number) {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.min(10, Math.max(-10, value));
}

function formatSubtitleDelay(value: number) {
  const delay = clampSubtitleDelay(value);
  if (Math.abs(delay) < 0.005) {
    return "0.0s";
  }

  return `${delay > 0 ? "+" : ""}${delay.toFixed(1)}s`;
}

function platformUnsupportedPlaybackMessage(support: PlatformSupport | null, t: AppStrings) {
  const session = [support?.os, support?.displayServer].filter(Boolean).join(" / ") || t.common.currentPlatform;
  return t.status.unsupportedPlayback(session);
}

function trackDisplayLabel(track: MpvTrack, t: AppStrings) {
  const title = track.title || `${track.kind.toUpperCase()} ${track.id}`;
  const details = [track.language?.toUpperCase(), track.codec, track.external ? t.common.external : null].filter(Boolean);
  return details.length ? `${title} · ${details.join(" · ")}` : title;
}

function snapEndOfMediaPosition(position: number, duration: number, isPlaying: boolean) {
  if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0) {
    return Number.isFinite(position) ? Math.max(0, position) : 0;
  }

  const clamped = Math.min(duration, Math.max(0, position));
  if (!isPlaying && duration - clamped <= END_OF_MEDIA_SNAP_TOLERANCE_SECONDS) {
    return duration;
  }

  return clamped;
}

function mediaNameFromPath(path: string) {
  const normalized = path.replace(/\\/g, "/");
  return normalized.split("/").pop() || path;
}

function defaultShellPreviewExtensions(formats: ShellPreviewFormatInfo[]) {
  return formats.filter((format) => format.common).map((format) => format.extension);
}

function isPlayableMediaPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return Boolean(extension && playableExtensions.includes(extension));
}

function isAudioMediaPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return Boolean(extension && audioOnlyExtensions.includes(extension));
}

function sortMediaPaths(paths: string[]) {
  return [...paths]
    .filter(isPlayableMediaPath)
    .sort((left, right) => mediaPathCollator.compare(mediaNameFromPath(left), mediaNameFromPath(right)) || mediaPathCollator.compare(left, right));
}

function uniqueMediaPaths(paths: string[], existingPaths: Set<string> = new Set()) {
  const seen = new Set(existingPaths);
  const unique: string[] = [];
  for (const path of sortMediaPaths(paths)) {
    if (seen.has(path)) {
      continue;
    }
    seen.add(path);
    unique.push(path);
  }
  return unique;
}

function mediaItemFromPath(path: string): MediaItem {
  return {
    id: nextMediaItemId(),
    name: mediaNameFromPath(path),
    path,
  };
}

function hwdecModeFromSnapshot(hwdec: string | null | undefined): HardwareDecodingMode {
  return hwdec?.trim().toLowerCase() === "no" ? "software" : "hardware";
}

function mediaItemFromHistory(entry: PlaybackHistoryEntry): MediaItem {
  return {
    id: nextMediaItemId(),
    name: entry.name || mediaNameFromPath(entry.path),
    path: entry.path,
  };
}

function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    close: "M6 6l12 12M18 6 6 18",
    cpu: "M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3M8 6h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2ZM10 10h4v4h-4z",
    folder: "M3 7.5h6l2 2h10v8.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5Z",
    folderAdd: "M3 7.5h6l2 2h10v8.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5ZM12 13v5M9.5 15.5h5",
    fullscreen: "M8 4H4v4M16 4h4v4M20 16v4h-4M8 20H4v-4",
    list: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
    maximize: "M7 7h10v10H7z",
    minimize: "M6 12h12",
    next: "M7 6l7 6-7 6V6ZM16 6v12",
    palette: "M12 3a9 9 0 0 0 0 18h1.2a1.8 1.8 0 0 0 1.3-3.05 1.8 1.8 0 0 1 1.27-3.07H18a3 3 0 0 0 3-3A9 9 0 0 0 12 3ZM7.5 11.5h.01M9 7.5h.01M14 7.5h.01M16.5 11h.01",
    pause: "M8 6h3v12H8zM13 6h3v12h-3z",
    play: "M8 5v14l11-7z",
    plugin: "M9 3v4M15 3v4M8 7h8a2 2 0 0 1 2 2v3a6 6 0 0 1-12 0V9a2 2 0 0 1 2-2ZM12 18v3",
    preview: "M4 5h16v11H4zM8 20h8M10 16l-1.5 4M14 16l1.5 4M7 13l3-3 2 2 2.5-3 3.5 4",
    previous: "M17 6l-7 6 7 6V6ZM8 6v12",
    restart: "M5 12a7 7 0 1 0 2-4.9M5 5v5h5",
    settings: "M12 8.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7ZM19 12a7.2 7.2 0 0 0-.08-1l2-1.55-2-3.45-2.36.95a7.4 7.4 0 0 0-1.72-1L14.5 3h-4l-.34 2.95a7.4 7.4 0 0 0-1.72 1L6.08 6l-2 3.45L6.08 11A7.2 7.2 0 0 0 6 12c0 .34.03.67.08 1l-2 1.55 2 3.45 2.36-.95c.53.42 1.1.75 1.72 1l.34 2.95h4l.34-2.95c.62-.25 1.19-.58 1.72-1l2.36.95 2-3.45-2-1.55c.05-.33.08-.66.08-1Z",
    stop: "M7 7h10v10H7z",
    tracks: "M4 7h7M15 7h5M11 5v4M4 12h12M20 12h0M16 10v4M4 17h4M12 17h8M8 15v4",
    volume: "M4 10v4h4l5 4V6l-5 4H4Z M16 9a4 4 0 0 1 0 6",
  };

  return (
    <svg aria-hidden="true" className="icon" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" viewBox="0 0 24 24">
      <path d={paths[name]} />
    </svg>
  );
}

function App() {
  if (surface === "video") {
    return <main className="video-host-surface" aria-label="OpenPlayer video surface" />;
  }

  const [queue, setQueue] = useState<MediaItem[]>([]);
  const [currentIndex, setCurrentIndex] = useState<number | null>(null);
  const [playbackHistory, setPlaybackHistory] = useState<PlaybackHistoryEntry[]>([]);
  const [isPlaylistOpen, setIsPlaylistOpen] = useState(false);
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [displayPosition, setDisplayPosition] = useState(0);
  const [volumeLevel, setVolumeLevel] = useState(0.82);
  const [playbackSpeed, setPlaybackSpeedValue] = useState(1);
  const [hardwareDecodingMode, setHardwareDecodingModeValue] = useState<HardwareDecodingMode>("hardware");
  const [isVideoFillEnabled, setIsVideoFillEnabled] = useState(false);
  const [subtitleDelay, setSubtitleDelayValue] = useState(0);
  const [tracks, setTracks] = useState<MpvTrack[]>([]);
  const [loadedMediaPath, setLoadedMediaPath] = useState<string | null>(null);
  const [framesPerSecond, setFramesPerSecond] = useState(0);
  const [timeDisplayMode, setTimeDisplayMode] = useState<TimeDisplayMode>("timecode");
  const [loopMode, setLoopMode] = useState<LoopMode>("off");
  const [isPlaying, setIsPlaying] = useState(false);
  const [isChromeVisible, setIsChromeVisible] = useState(true);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [platformSupport, setPlatformSupport] = useState<PlatformSupport | null>(null);
  const [appearanceState, setAppearanceState] = useState<AppearanceState | null>(null);
  const [playerPreferences, setPlayerPreferences] = useState<PlayerPreferences>(DEFAULT_PLAYER_PREFERENCES);
  const [volumeFeedback, setVolumeFeedback] = useState<VolumeFeedback | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SettingsSection>("appearance");
  const [mediaPanelMode, setMediaPanelMode] = useState<MediaPanelMode | null>(null);
  const [resizeFeedback, setResizeFeedback] = useState<ResizeFeedback | null>(null);
  const [shellPreviewFormats, setShellPreviewFormats] = useState<ShellPreviewFormatInfo[]>([]);
  const [selectedShellPreviewFormats, setSelectedShellPreviewFormats] = useState<string[]>([]);
  const [shellPreviewRegistrationStatus, setShellPreviewRegistrationStatus] = useState<string | null>(null);
  const [isRegisteringShellPreview, setIsRegisteringShellPreview] = useState(false);
  const [shortcutBindings, setShortcutBindings] = useState<ShortcutBindings>(readShortcutBindings);
  const [recordingShortcutAction, setRecordingShortcutAction] = useState<ShortcutAction | null>(null);
  const pendingSeekRef = useRef<PendingSeek | null>(null);
  const playbackClockAnchorRef = useRef<PlaybackClockAnchor>({ position: 0, startedAt: performance.now(), playing: false, speed: 1 });
  const snapshotRequestIdRef = useRef(0);
  const chromeHideTimerRef = useRef<number | null>(null);
  const volumeFeedbackTimerRef = useRef<number | null>(null);
  const pendingWindowDragRef = useRef<PendingWindowDrag | null>(null);
  const manualResizeDragRef = useRef<ManualResizeDrag | null>(null);
  const resizeCursorDirectionRef = useRef<ResizeDirection | null>(null);
  const handledEndedPathRef = useRef<string | null>(null);
  const lastHistoryWriteRef = useRef(0);
  const hardwareDecodingModeRef = useRef<HardwareDecodingMode>("hardware");
  const shortcutKeyDownRef = useRef<(event: KeyboardEvent) => void>(() => undefined);
  const nativeShortcutActionRef = useRef<(action: ShortcutAction) => void>(() => undefined);
  const settingsDialogRef = useRef<HTMLElement | null>(null);
  const media = currentIndex === null ? null : (queue[currentIndex] ?? null);
  const locale = resolveLocale(playerPreferences.languageMode, browserLanguages());
  const t = translations[locale];
  const loopModeOptions = loopModeOptionsFor(t);
  const shortcutDefinitions = shortcutDefinitionsFor(t);
  const activeTheme = activeThemeFromAppearance(appearanceState);
  const appearanceStyle = themeStyleVariables(appearanceState);
  const isMediaPanelOpen = mediaPanelMode !== null;
  const isChromePinned = !media || isPlaylistOpen || isMediaPanelOpen || isPickerOpen || playbackError !== null || contextMenu !== null || isSettingsOpen;

  useEffect(() => {
    let disposed = false;
    invoke<PlatformSupport>("platform_support")
      .then((support) => {
        if (!disposed) {
          setPlatformSupport(support);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load platform support metadata", error);
      });

    invoke<PlaybackHistoryEntry[]>("history_list")
      .then((entries) => {
        if (!disposed) {
          setPlaybackHistory(Array.isArray(entries) ? entries : []);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load playback history", error);
      });

    invoke<AppearanceState>("appearance_state")
      .then((state) => {
        if (!disposed) {
          setAppearanceState(state);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load appearance settings", error);
      });

    invoke<PlayerPreferences>("preferences_state")
      .then((preferences) => {
        if (!disposed) {
          setPlayerPreferences(preferences);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load player preferences", error);
      });

    invoke<ShellPreviewFormatInfo[]>("shell_preview_formats")
      .then((formats) => {
        if (!disposed && Array.isArray(formats)) {
          setShellPreviewFormats(formats);
          setSelectedShellPreviewFormats(defaultShellPreviewExtensions(formats));
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load Explorer preview formats", error);
      });

    invoke<string[]>("startup_media_paths")
      .then((paths) => {
        if (!disposed && Array.isArray(paths) && paths.length > 0) {
          replaceQueueWithMediaPaths(paths).catch(reportPlaybackError);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load startup media paths", error);
      });

    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    if (!media) {
      return;
    }

    const timer = window.setInterval(() => {
      const requestId = ++snapshotRequestIdRef.current;
      invoke<MpvSnapshot | null>("mpv_embed_snapshot")
        .then((snapshot) => {
          if (snapshot && requestId === snapshotRequestIdRef.current) {
            applySnapshot(snapshot);
          }
        })
        .catch(() => undefined);
    }, 500);

    return () => {
      window.clearInterval(timer);
      invalidatePendingSnapshots();
    };
  }, [media?.id]);

  useEffect(() => {
    if (!media || !isPlaying || duration <= 0) {
      return;
    }

    let frameId = 0;
    const tick = () => {
      const anchor = playbackClockAnchorRef.current;
      const elapsedSeconds = anchor.playing ? (performance.now() - anchor.startedAt) / 1000 : 0;
      setDisplayPosition(clampPlaybackPosition(anchor.position + elapsedSeconds * anchor.speed, duration));
      frameId = window.requestAnimationFrame(tick);
    };

    frameId = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frameId);
  }, [media?.id, isPlaying, duration]);

  useEffect(() => {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayMode("timecode");
    }
  }, [framesPerSecond, duration]);

  useEffect(() => {
    if (!media || loadedMediaPath !== media.path) {
      return;
    }

    invoke<MpvSnapshot>("mpv_embed_set_loop_file", { enabled: loopMode === "one" })
      .then((snapshot) => {
        if (loopMode === "one") {
          applySnapshot(snapshot);
        }
      })
      .catch(reportPlaybackError);
  }, [media?.id, media?.path, loadedMediaPath, loopMode]);

  useEffect(() => {
    setIsChromeVisible(true);
    scheduleChromeHide();
    return () => {
      clearChromeHideTimer();
      clearPendingWindowDrag();
      clearManualResizeDrag();
      setNativeResizeCursor(null);
      setResizeFeedback(null);
    };
  }, [media?.id, isChromePinned]);

  useEffect(() => {
    try {
      window.localStorage.setItem(OPENPLAYER_SHORTCUTS_STORAGE_KEY, JSON.stringify(shortcutBindings));
    } catch (error) {
      console.warn("Failed to persist shortcut settings", error);
    }
  }, [shortcutBindings]);

  useEffect(() => {
    if (isSettingsOpen) {
      settingsDialogRef.current?.focus();
    } else {
      setRecordingShortcutAction(null);
    }
  }, [isSettingsOpen]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      invoke<AppearanceState>("appearance_state")
        .then(setAppearanceState)
        .catch((error: unknown) => console.warn("Failed to sync appearance settings", error));
      invoke<PlayerPreferences>("preferences_state")
        .then(setPlayerPreferences)
        .catch((error: unknown) => console.warn("Failed to sync player preferences", error));
      invoke<PlaybackHistoryEntry[]>("history_list")
        .then((entries) => setPlaybackHistory(Array.isArray(entries) ? entries : []))
        .catch((error: unknown) => console.warn("Failed to sync playback history", error));
    }, STORE_SYNC_INTERVAL_MS);

    return () => window.clearInterval(timer);
  }, []);

  shortcutKeyDownRef.current = (event: KeyboardEvent) => {
    if (recordingShortcutAction) {
      recordUserActivity();
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setRecordingShortcutAction(null);
        return;
      }

      if (event.key === "Backspace" || event.key === "Delete") {
        assignShortcut(recordingShortcutAction, null);
        setRecordingShortcutAction(null);
        return;
      }

      const chord = keyboardEventToChord(event);
      if (chord) {
        assignShortcut(recordingShortcutAction, chord);
        setRecordingShortcutAction(null);
      }
      return;
    }

    if (event.key === "Escape") {
      recordUserActivity();
      if (contextMenu) {
        event.preventDefault();
        setContextMenu(null);
        return;
      }

      if (isSettingsOpen) {
        event.preventDefault();
        setIsSettingsOpen(false);
      }
      return;
    }

    if (contextMenu || isSettingsOpen || isTextEntryShortcutTarget(event.target)) {
      return;
    }

    const chord = keyboardEventToChord(event);
    const shortcut = shortcutDefinitions.find((definition) => shortcutBindings[definition.action] === chord);
    if (!shortcut) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    releaseShortcutFocusTarget(event.target);
    recordShortcutActivity(shortcut.action);
    performShortcutAction(shortcut.action);
  };

  nativeShortcutActionRef.current = (action: ShortcutAction) => {
    if (contextMenu || isSettingsOpen || recordingShortcutAction) {
      return;
    }

    recordShortcutActivity(action);
    performShortcutAction(action);
  };

  useEffect(() => {
    function handleGlobalKeyDown(event: KeyboardEvent) {
      shortcutKeyDownRef.current(event);
    }

    window.addEventListener("keydown", handleGlobalKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", handleGlobalKeyDown, { capture: true });
  }, []);

  useEffect(() => {
    let unlistenShortcut: (() => void) | null = null;
    let disposed = false;

    listen<string>("openplayer-native-shortcut", (event) => {
      if (isShortcutAction(event.payload)) {
        nativeShortcutActionRef.current(event.payload);
      }
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenShortcut = unlisten;
        }
      })
      .catch((error: unknown) => {
        console.warn("Native shortcut listener failed", error);
      });

    return () => {
      disposed = true;
      unlistenShortcut?.();
    };
  }, []);

  useEffect(() => {
    invoke("window_update_shortcuts", { bindings: shortcutBindings }).catch((error: unknown) => {
      console.warn("Native shortcut update failed", error);
    });
  }, [shortcutBindings]);

  useEffect(() => {
    const enabled = !contextMenu && !isSettingsOpen && !recordingShortcutAction;
    invoke("window_set_shortcuts_enabled", { enabled }).catch((error: unknown) => {
      console.warn("Native shortcut state update failed", error);
    });
  }, [contextMenu, isSettingsOpen, recordingShortcutAction]);

  useEffect(() => {
    return () => {
      if (volumeFeedbackTimerRef.current !== null) {
        window.clearTimeout(volumeFeedbackTimerRef.current);
      }
    };
  }, []);

  function clearChromeHideTimer() {
    if (chromeHideTimerRef.current !== null) {
      window.clearTimeout(chromeHideTimerRef.current);
      chromeHideTimerRef.current = null;
    }
  }

  function clearPendingWindowDrag() {
    pendingWindowDragRef.current = null;
  }

  function clearManualResizeDrag() {
    const pendingResize = manualResizeDragRef.current;
    if (pendingResize?.animationFrameId != null) {
      window.cancelAnimationFrame(pendingResize.animationFrameId);
    }
    manualResizeDragRef.current = null;
  }

  function setNativeResizeCursor(direction: ResizeDirection | null) {
    if (resizeCursorDirectionRef.current === direction) {
      return;
    }

    resizeCursorDirectionRef.current = direction;
    void applyResizeCursor(direction);
  }

  function setResizeBoundaryFeedback(direction: ResizeDirection | null, active = false) {
    setResizeFeedback((feedback) => {
      if (!direction) {
        return feedback === null ? feedback : null;
      }

      if (feedback?.direction === direction && feedback.active === active) {
        return feedback;
      }

      return { direction, active };
    });
  }

  function completeManualResizeIfIdle(pendingResize: ManualResizeDrag) {
    if (
      pendingResize.finishing &&
      !pendingResize.resizeCommandInFlight &&
      pendingResize.animationFrameId === null &&
      Math.abs(pendingResize.pendingDeltaX) < 0.5 &&
      Math.abs(pendingResize.pendingDeltaY) < 0.5 &&
      manualResizeDragRef.current === pendingResize
    ) {
      manualResizeDragRef.current = null;
    }
  }

  function requestManualResizeFlush() {
    const pendingResize = manualResizeDragRef.current;
    if (!pendingResize || pendingResize.animationFrameId !== null || pendingResize.resizeCommandInFlight) {
      return;
    }

    pendingResize.animationFrameId = window.requestAnimationFrame(() => {
      const activeResize = manualResizeDragRef.current;
      if (!activeResize) {
        return;
      }

      activeResize.animationFrameId = null;
      flushManualResizeDelta();
    });
  }

  function flushManualResizeDelta() {
    const pendingResize = manualResizeDragRef.current;
    if (!pendingResize || pendingResize.resizeCommandInFlight) {
      return;
    }

    const deltaX = pendingResize.pendingDeltaX;
    const deltaY = pendingResize.pendingDeltaY;
    if (Math.abs(deltaX) < 0.5 && Math.abs(deltaY) < 0.5) {
      completeManualResizeIfIdle(pendingResize);
      return;
    }

    pendingResize.pendingDeltaX = 0;
    pendingResize.pendingDeltaY = 0;
    pendingResize.resizeCommandInFlight = true;
    applyManualMainWindowResize(pendingResize.direction, deltaX, deltaY).finally(() => {
      if (manualResizeDragRef.current !== pendingResize) {
        return;
      }

      pendingResize.resizeCommandInFlight = false;
      if (Math.abs(pendingResize.pendingDeltaX) >= 0.5 || Math.abs(pendingResize.pendingDeltaY) >= 0.5) {
        requestManualResizeFlush();
        return;
      }

      completeManualResizeIfIdle(pendingResize);
    });
  }

  function scheduleChromeHide() {
    clearChromeHideTimer();
    if (isChromePinned) {
      return;
    }

    chromeHideTimerRef.current = window.setTimeout(() => {
      setIsChromeVisible(false);
      chromeHideTimerRef.current = null;
    }, AUTO_HIDE_CONTROLS_MS);
  }

  function recordUserActivity() {
    setIsChromeVisible(true);
    scheduleChromeHide();
  }

  function recordShortcutActivity(action: ShortcutAction) {
    if (playerPreferences.quietKeyboardControls && ["seekBackward", "seekForward", "volumeDown", "volumeUp"].includes(action)) {
      return;
    }

    recordUserActivity();
  }

  function showVolumeFeedback(level: number) {
    const nextLevel = Math.min(1, Math.max(0, level));
    setVolumeFeedback({ level: nextLevel });
    if (volumeFeedbackTimerRef.current !== null) {
      window.clearTimeout(volumeFeedbackTimerRef.current);
    }
    volumeFeedbackTimerRef.current = window.setTimeout(() => {
      setVolumeFeedback(null);
      volumeFeedbackTimerRef.current = null;
    }, VOLUME_FEEDBACK_MS);
  }

  function invalidatePendingSnapshots() {
    snapshotRequestIdRef.current += 1;
  }

  function applyCommandSnapshot(snapshot: MpvSnapshot) {
    invalidatePendingSnapshots();
    applySnapshot(snapshot, true);
  }

  function applySnapshot(snapshot: MpvSnapshot, forceHistoryWrite = false) {
    const snapshotPosition = Number.isFinite(snapshot.position) ? snapshot.position : 0;
    const snapshotDuration = Number.isFinite(snapshot.duration) ? snapshot.duration : 0;
    const snapshotSpeed = clampPlaybackSpeed(snapshot.speed);
    const pendingSeek = pendingSeekRef.current;
    const nextIsPlaying = !snapshot.paused && snapshot.status === "playing";

    setDuration(snapshotDuration);
    setIsPlaying(nextIsPlaying);
    setFramesPerSecond(Number.isFinite(snapshot.fps) && snapshot.fps > 0 ? snapshot.fps : 0);
    setPlaybackSpeedValue(snapshotSpeed);
    setHardwareDecodingModeValue(hwdecModeFromSnapshot(snapshot.hwdec));
    hardwareDecodingModeRef.current = hwdecModeFromSnapshot(snapshot.hwdec);
    setIsVideoFillEnabled(snapshot.videoFill === true);
    setSubtitleDelayValue(clampSubtitleDelay(snapshot.subtitleDelay));
    setTracks(Array.isArray(snapshot.tracks) ? snapshot.tracks : []);
    setLoadedMediaPath(snapshot.path);
    setVolumeLevel(Math.min(1, Math.max(0, snapshot.volume / 100)));

    if (pendingSeek) {
      const isConfirmed = Math.abs(snapshotPosition - pendingSeek.target) <= SEEK_CONFIRM_TOLERANCE_SECONDS;
      const isExpired = performance.now() - pendingSeek.startedAt > SEEK_SNAPSHOT_SUPPRESS_MS;
      if (!isConfirmed && !isExpired) {
        return;
      }

      pendingSeekRef.current = null;
    }

    rememberPlaybackProgress(snapshot.path, snapshotPosition, snapshotDuration, forceHistoryWrite);
    setCurrentTime(snapshotPosition);
    anchorDisplayClock(snapshotPosition, nextIsPlaying, snapshotDuration, snapshotSpeed);

    if (snapshot.ended || snapshot.status === "ended") {
      handlePlaybackEnd(snapshot.path);
    } else if (handledEndedPathRef.current === snapshot.path) {
      handledEndedPathRef.current = null;
    }
  }

  function rememberPlaybackProgress(path: string, position: number, snapshotDuration: number, force = false) {
    if (!path || playerPreferences.incognitoMode) {
      return;
    }

    const now = Date.now();
    if (!force && now - lastHistoryWriteRef.current < HISTORY_WRITE_INTERVAL_MS) {
      return;
    }

    lastHistoryWriteRef.current = now;
    invoke<PlaybackHistoryEntry[]>("history_remember", {
      entry: {
        path,
        name: mediaNameFromPath(path),
        position: Number.isFinite(position) ? Math.max(0, position) : 0,
        duration: Number.isFinite(snapshotDuration) ? Math.max(0, snapshotDuration) : 0,
        updatedAt: now,
      },
    })
      .then((entries) => setPlaybackHistory(Array.isArray(entries) ? entries : []))
      .catch((error: unknown) => {
        console.warn("Failed to remember playback progress", error);
      });
  }

  function reportPlaybackError(error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.includes("mpv has no loaded media")) {
      return;
    }

    if (
      message.includes("mpv embed playback currently supports Windows HWND hosts only") ||
      message.includes("video host support is not implemented yet")
    ) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return;
    }

    setPlaybackError(message);
  }

  async function openMpvPath(path: string) {
    invalidatePendingSnapshots();
    handledEndedPathRef.current = null;
    setLoadedMediaPath(null);
    const rememberedPosition = await resumePositionForPath(path);
    const snapshot = await invoke<MpvSnapshot>("mpv_overlay_open_path", { path });
    pendingSeekRef.current = null;
    setPlaybackError(null);
    applyCommandSnapshot(snapshot);
    let activeSnapshot = snapshot;
    if (hardwareDecodingModeRef.current === "software") {
      activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_hwdec", { mode: "software" });
      applyCommandSnapshot(activeSnapshot);
    }

    const resumeTarget = resumePositionWithinDuration(rememberedPosition, activeSnapshot.duration);
    if (resumeTarget <= 0) {
      return;
    }

    pendingSeekRef.current = { target: resumeTarget, startedAt: performance.now() };
    setCurrentTime(resumeTarget);
    anchorDisplayClock(resumeTarget, false, activeSnapshot.duration, activeSnapshot.speed);
    invalidatePendingSnapshots();
    const resumedSnapshot = await invoke<MpvSnapshot>("mpv_embed_seek", { position: resumeTarget });
    applyCommandSnapshot(resumedSnapshot);
  }

  function seekTarget(value: number) {
    if (!Number.isFinite(value)) {
      return 0;
    }

    const upperBound = duration > 0 ? duration : value;
    return Math.min(upperBound, Math.max(0, value));
  }

  function clampPlaybackPosition(value: number, upperDuration = duration) {
    if (!Number.isFinite(value)) {
      return 0;
    }

    const upperBound = upperDuration > 0 ? upperDuration : value;
    return Math.min(upperBound, Math.max(0, value));
  }

  function anchorDisplayClock(position: number, playing: boolean, upperDuration = duration, speed = playbackSpeed) {
    const clampedPosition = clampPlaybackPosition(position, upperDuration);
    playbackClockAnchorRef.current = {
      position: clampedPosition,
      startedAt: performance.now(),
      playing,
      speed: clampPlaybackSpeed(speed),
    };
    setDisplayPosition(clampedPosition);
  }

  function toggleTimeDisplayMode() {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayMode("timecode");
      return;
    }

    setTimeDisplayMode((mode) => (mode === "timecode" ? "frames" : "timecode"));
  }

  function assignShortcut(action: ShortcutAction, chord: string | null) {
    setShortcutBindings((bindings) => {
      const next = { ...bindings, [action]: chord };
      if (chord) {
        for (const definition of shortcutDefinitions) {
          if (definition.action !== action && next[definition.action] === chord) {
            next[definition.action] = null;
          }
        }
      }

      return next;
    });
  }

  function resetShortcutBindings() {
    setShortcutBindings(defaultShortcutBindings);
    setRecordingShortcutAction(null);
  }

  async function updateAppearance(request: Promise<AppearanceState>) {
    try {
      setAppearanceState(await request);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      focusOverlayWindow();
    }
  }

  function selectTheme(themeId: string) {
    void updateAppearance(invoke<AppearanceState>("appearance_set_theme", { themeId }));
  }

  function setAccentOverride(accent: string | null) {
    void updateAppearance(invoke<AppearanceState>("appearance_set_accent_override", { accent }));
  }

  function resetAppearance() {
    void updateAppearance(invoke<AppearanceState>("appearance_reset"));
  }

  function setThemePluginEnabled(pluginId: string, enabled: boolean) {
    void updateAppearance(invoke<AppearanceState>("appearance_set_plugin_enabled", { pluginId, enabled }));
  }

  async function updatePlayerPreferences(request: Promise<PlayerPreferences>) {
    try {
      setPlayerPreferences(await request);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      focusOverlayWindow();
    }
  }

  function setIncognitoMode(enabled: boolean) {
    void updatePlayerPreferences(invoke<PlayerPreferences>("preferences_set_incognito_mode", { enabled }));
  }

  function setQuietKeyboardControls(enabled: boolean) {
    void updatePlayerPreferences(invoke<PlayerPreferences>("preferences_set_quiet_keyboard_controls", { enabled }));
  }

  function setLanguageMode(mode: LanguageMode) {
    void updatePlayerPreferences(invoke<PlayerPreferences>("preferences_set_language_mode", { mode }));
  }

  function toggleShellPreviewFormat(extension: string) {
    setShellPreviewRegistrationStatus(null);
    setSelectedShellPreviewFormats((selected) => {
      if (selected.includes(extension)) {
        return selected.filter((item) => item !== extension);
      }

      return [...selected, extension];
    });
  }

  function toggleAllShellPreviewFormats() {
    setShellPreviewRegistrationStatus(null);
    setSelectedShellPreviewFormats((selected) => {
      if (shellPreviewFormats.length > 0 && selected.length === shellPreviewFormats.length) {
        return [];
      }

      return shellPreviewFormats.map((format) => format.extension);
    });
  }

  function resetShellPreviewFormatsToDefault() {
    setShellPreviewRegistrationStatus(null);
    setSelectedShellPreviewFormats(defaultShellPreviewExtensions(shellPreviewFormats));
  }

  async function registerShellPreviews() {
    if (isRegisteringShellPreview) {
      return;
    }

    if (!selectedShellPreviewFormats.length) {
      setShellPreviewRegistrationStatus(t.settings.shellPreview.noSelection);
      return;
    }

    setIsRegisteringShellPreview(true);
    setShellPreviewRegistrationStatus(null);
    try {
      const summary = await invoke<ShellPreviewRegistrationSummary>("shell_preview_register_formats", { selectedExtensions: selectedShellPreviewFormats });
      setShellPreviewRegistrationStatus(t.settings.shellPreview.registered(summary.registeredCount));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setShellPreviewRegistrationStatus(t.settings.shellPreview.failed(message));
    } finally {
      setIsRegisteringShellPreview(false);
      focusOverlayWindow();
    }
  }

  async function openDefaultAppsSettings() {
    try {
      await invoke("shell_preview_open_default_apps_settings");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setShellPreviewRegistrationStatus(t.settings.shellPreview.openDefaultAppsFailed(message));
    }
  }

  async function importThemePlugin() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        multiple: false,
        filters: [{ name: t.dialog.themePlugin, extensions: themePluginExtensions }],
      });
      if (typeof selection !== "string") {
        return;
      }

      await updateAppearance(invoke<AppearanceState>("appearance_import_theme_plugin", { path: selection }));
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  function openSettingsDialog() {
    setContextMenu(null);
    setMediaPanelMode(null);
    setSettingsSection("appearance");
    setIsSettingsOpen(true);
  }

  function closeSettingsDialog() {
    setIsSettingsOpen(false);
  }

  function openContextMenu(event: ReactMouseEvent<HTMLElement>) {
    event.preventDefault();
    recordUserActivity();
    const x = Math.min(Math.max(8, event.clientX), Math.max(8, window.innerWidth - CONTEXT_MENU_WIDTH - 8));
    const y = Math.min(Math.max(8, event.clientY), Math.max(8, window.innerHeight - CONTEXT_MENU_HEIGHT - 8));
    setContextMenu({ x, y });
  }

  function handleShellPointerDown() {
    recordUserActivity();
    if (contextMenu) {
      setContextMenu(null);
    }
  }

  function handleShellPointerLeave() {
    clearChromeHideTimer();
    if (!manualResizeDragRef.current) {
      setNativeResizeCursor(null);
      setResizeBoundaryFeedback(null);
    }
    if (media && !isChromePinned) {
      setIsChromeVisible(false);
    }
  }

  function handleShellWheel(event: ReactWheelEvent<HTMLElement>) {
    if (!media || contextMenu || isSettingsOpen || recordingShortcutAction) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const direction = event.deltaY > 0 ? -1 : 1;
    setVolume(volumeLevel + direction * DEFAULT_VOLUME_STEP, { feedback: true });
  }

  function seekBy(deltaSeconds: number) {
    if (!media || duration <= 0) {
      return;
    }

    commitSeekTo(displayTime + deltaSeconds);
  }

  function stepFrame(command: "mpv_embed_frame_step" | "mpv_embed_frame_back_step") {
    if (!media) {
      return;
    }

    pendingSeekRef.current = null;
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>(command).then(applyCommandSnapshot).catch(reportPlaybackError);
  }

  function performShortcutAction(action: ShortcutAction) {
    switch (action) {
      case "openMedia":
        openNativeMediaFiles();
        break;
      case "togglePlayback":
        togglePlayback();
        break;
      case "restart":
        if (media) {
          restartPlayback();
        }
        break;
      case "togglePlaylist":
        if (media || queue.length > 0) {
          togglePlaylist();
        }
        break;
      case "seekBackward":
        seekBy(-DEFAULT_SEEK_STEP_SECONDS);
        break;
      case "seekForward":
        seekBy(DEFAULT_SEEK_STEP_SECONDS);
        break;
      case "frameForward":
        stepFrame("mpv_embed_frame_step");
        break;
      case "frameBackward":
        stepFrame("mpv_embed_frame_back_step");
        break;
      case "volumeDown":
        setVolume(volumeLevel - DEFAULT_VOLUME_STEP, { feedback: true });
        break;
      case "volumeUp":
        setVolume(volumeLevel + DEFAULT_VOLUME_STEP, { feedback: true });
        break;
      case "toggleFullscreen":
        toggleFullscreen();
        break;
      case "openSettings":
        openSettingsDialog();
        break;
    }
  }

  async function openNativeMediaFiles() {
    if (isPickerOpen) {
      return;
    }

    if (platformSupport && !platformSupport.mpvEmbedVideo) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: t.dialog.mediaFiles, extensions: playableExtensions }],
      });
      const paths = typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
      await replaceQueueWithMediaPaths(paths);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function appendNativeMediaFiles() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: t.dialog.mediaFiles, extensions: playableExtensions }],
      });
      const paths = typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
      await appendMediaPaths(paths);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function appendNativeMediaFolder() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        directory: true,
        multiple: false,
      });
      const folderPath = typeof selection === "string" ? selection : null;
      if (!folderPath) {
        return;
      }

      const paths = await invoke<string[]>("media_files_in_directory", { path: folderPath });
      await appendMediaPaths(paths);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function replaceQueueWithMediaPaths(paths: string[]) {
    if (platformSupport && !platformSupport.mpvEmbedVideo) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return;
    }

    const nextQueue = uniqueMediaPaths(paths).map(mediaItemFromPath);
    if (!nextQueue.length) {
      return;
    }

    setQueue(nextQueue);
    setCurrentIndex(0);
    setIsPlaylistOpen(nextQueue.length > 1);
    await openMpvPath(nextQueue[0].path);
  }

  async function appendMediaPaths(paths: string[]) {
    if (platformSupport && !platformSupport.mpvEmbedVideo) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return;
    }

    const baseQueue = queue.length ? queue : media ? [media] : [];
    const appendedPaths = uniqueMediaPaths(paths, new Set(baseQueue.map((item) => item.path)));
    if (!appendedPaths.length) {
      return;
    }

    const nextQueue = [...baseQueue, ...appendedPaths.map(mediaItemFromPath)];
    const shouldStartPlayback = !media;
    setQueue(nextQueue);
    setCurrentIndex(shouldStartPlayback ? 0 : currentIndex ?? 0);
    setIsPlaylistOpen(nextQueue.length > 1);
    if (shouldStartPlayback) {
      await openMpvPath(nextQueue[0].path);
    }
  }

  function playQueueIndex(index: number) {
    const item = queue[index];
    if (!item) {
      return;
    }

    handledEndedPathRef.current = null;
    setCurrentIndex(index);
    openMpvPath(item.path).catch(reportPlaybackError);
  }

  function chooseQueueItem(index: number) {
    if (index === currentIndex) {
      return;
    }

    playQueueIndex(index);
  }

  function previousQueueIndex() {
    if (currentIndex === null || !queue.length) {
      return null;
    }
    if (currentIndex > 0) {
      return currentIndex - 1;
    }
    return loopMode === "all" && queue.length > 1 ? queue.length - 1 : null;
  }

  function nextQueueIndex() {
    if (currentIndex === null || !queue.length) {
      return null;
    }
    if (currentIndex < queue.length - 1) {
      return currentIndex + 1;
    }
    return loopMode === "all" && queue.length > 1 ? 0 : null;
  }

  function playPreviousQueueItem() {
    const index = previousQueueIndex();
    if (index !== null) {
      playQueueIndex(index);
    }
  }

  function playNextQueueItem() {
    const index = nextQueueIndex();
    if (index !== null) {
      playQueueIndex(index);
    }
  }

  function openHistoryEntry(entry: PlaybackHistoryEntry) {
    const item = mediaItemFromHistory(entry);
    setQueue([item]);
    setCurrentIndex(0);
    setIsPlaylistOpen(false);
    openMpvPath(entry.path).catch(reportPlaybackError);
  }

  function clearPlaybackHistory() {
    invoke<PlaybackHistoryEntry[]>("history_clear")
      .then((entries) => setPlaybackHistory(Array.isArray(entries) ? entries : []))
      .catch(reportPlaybackError);
  }

  function stopPlayback() {
    if (!media) {
      return;
    }

    invalidatePendingSnapshots();
    invoke<void>("mpv_embed_stop")
      .then(() => {
        handledEndedPathRef.current = null;
        pendingSeekRef.current = null;
        setCurrentIndex(null);
        setIsPlaying(false);
        setDuration(0);
        setCurrentTime(0);
        setDisplayPosition(0);
        setFramesPerSecond(0);
        setTracks([]);
        setLoadedMediaPath(null);
        setMediaPanelMode(null);
      })
      .catch(reportPlaybackError);
  }

  function restartPlayback(autoplay = false) {
    if (!media) {
      return;
    }

    pendingSeekRef.current = { target: 0, startedAt: performance.now() };
    setCurrentTime(0);
    anchorDisplayClock(0, false, duration, playbackSpeed);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_seek", { position: 0 })
      .then((snapshot) => {
        applyCommandSnapshot(snapshot);
        if (autoplay) {
          return invoke<MpvSnapshot>("mpv_embed_play").then((playingSnapshot) => {
            handledEndedPathRef.current = null;
            applyCommandSnapshot(playingSnapshot);
          });
        }
        return undefined;
      })
      .catch((error: unknown) => {
        pendingSeekRef.current = null;
        reportPlaybackError(error);
      });
  }

  function handlePlaybackEnd(path: string) {
    if (!media || media.path !== path || handledEndedPathRef.current === path) {
      return;
    }

    if (loopMode === "off") {
      return;
    }

    handledEndedPathRef.current = path;
    if (loopMode === "one") {
      restartPlayback(true);
      return;
    }

    const index = nextQueueIndex();
    if (index !== null) {
      playQueueIndex(index);
    } else {
      restartPlayback(true);
    }
  }

  function toggleFullscreen() {
    runWindowCommand("window_toggle_fullscreen");
  }

  function handleDragRegionPointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button === 1) {
      event.preventDefault();
      runWindowCommand("window_toggle_fullscreen");
      return;
    }

    if (event.button === 0) {
      if (event.detail > 1) {
        event.preventDefault();
        clearPendingWindowDrag();
        return;
      }
      pendingWindowDragRef.current = {
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
      };
    }
  }

  function handleDragRegionPointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    const pendingDrag = pendingWindowDragRef.current;
    if (!pendingDrag || pendingDrag.pointerId !== event.pointerId) {
      return;
    }

    const deltaX = event.clientX - pendingDrag.startX;
    const deltaY = event.clientY - pendingDrag.startY;
    if (deltaX * deltaX + deltaY * deltaY < WINDOW_DRAG_START_DISTANCE_PX * WINDOW_DRAG_START_DISTANCE_PX) {
      return;
    }

    event.preventDefault();
    clearPendingWindowDrag();
    startMainWindowDrag();
  }

  function handleDragRegionPointerEnd(event: ReactPointerEvent<HTMLDivElement>) {
    if (pendingWindowDragRef.current?.pointerId !== event.pointerId) {
      return;
    }

    clearPendingWindowDrag();
  }

  function handleDragRegionDoubleClick(event: ReactMouseEvent<HTMLDivElement>) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    clearPendingWindowDrag();
    togglePlayback();
  }

  function startMainWindowResize(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    setNativeResizeCursor(direction);
    setResizeBoundaryFeedback(direction, true);
    if (platformSupport?.os === "macos") {
      event.currentTarget.setPointerCapture(event.pointerId);
      manualResizeDragRef.current = {
        pointerId: event.pointerId,
        direction,
        lastX: event.clientX,
        lastY: event.clientY,
        pendingDeltaX: 0,
        pendingDeltaY: 0,
        animationFrameId: null,
        resizeCommandInFlight: false,
        finishing: false,
      };
      return;
    }

    startNativeMainWindowResize(direction);
  }

  function handleResizePointerEnter(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    event.stopPropagation();
    setNativeResizeCursor(direction);
    setResizeBoundaryFeedback(direction);
  }

  function handleResizePointerLeave(event: ReactPointerEvent<HTMLDivElement>) {
    if (manualResizeDragRef.current?.pointerId === event.pointerId) {
      return;
    }

    event.stopPropagation();
    setNativeResizeCursor(null);
    setResizeBoundaryFeedback(null);
  }

  function handleResizePointerDown(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    recordUserActivity();
    startMainWindowResize(event, direction);
  }

  function handleResizePointerMove(event: ReactPointerEvent<HTMLDivElement>) {
    const pendingResize = manualResizeDragRef.current;
    if (!pendingResize || pendingResize.pointerId !== event.pointerId) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    recordUserActivity();
    const scale = window.devicePixelRatio || 1;
    const deltaX = (event.clientX - pendingResize.lastX) * scale;
    const deltaY = (event.clientY - pendingResize.lastY) * scale;
    if (Math.abs(deltaX) < 0.5 && Math.abs(deltaY) < 0.5) {
      return;
    }
    pendingResize.lastX = event.clientX;
    pendingResize.lastY = event.clientY;
    pendingResize.pendingDeltaX += deltaX;
    pendingResize.pendingDeltaY += deltaY;
    requestManualResizeFlush();
  }

  function handleResizePointerEnd(event: ReactPointerEvent<HTMLDivElement>) {
    const pendingResize = manualResizeDragRef.current;
    if (pendingResize?.pointerId !== event.pointerId) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    pendingResize.finishing = true;
    requestManualResizeFlush();
    completeManualResizeIfIdle(pendingResize);
    setNativeResizeCursor(null);
    setResizeBoundaryFeedback(null);
  }

  function togglePlayback() {
    if (!media) {
      openNativeMediaFiles();
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>(isPlaying ? "mpv_embed_pause" : "mpv_embed_play")
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  function togglePlaylist() {
    setMediaPanelMode(null);
    setIsPlaylistOpen((isOpen) => !isOpen);
  }

  function toggleSpeedPanel() {
    setIsPlaylistOpen(false);
    setMediaPanelMode((mode) => (mode === "speed" ? null : "speed"));
  }

  function toggleTrackPanel() {
    setIsPlaylistOpen(false);
    setMediaPanelMode((mode) => (mode === "tracks" ? null : "tracks"));
  }

  function toggleLoopPanel() {
    setIsPlaylistOpen(false);
    setMediaPanelMode((mode) => (mode === "loop" ? null : "loop"));
  }

  function seekTo(value: number) {
    const target = seekTarget(value);
    pendingSeekRef.current = { target, startedAt: performance.now() };
    setCurrentTime(target);
    anchorDisplayClock(target, false);
  }

  function commitSeekTo(value: number) {
    const target = seekTarget(value);
    pendingSeekRef.current = { target, startedAt: performance.now() };
    setCurrentTime(target);
    anchorDisplayClock(target, false);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_seek", { position: target })
      .then(applyCommandSnapshot)
      .catch((error: unknown) => {
        pendingSeekRef.current = null;
        reportPlaybackError(error);
      });
  }

  function setVolume(value: number, options: { feedback?: boolean } = {}) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
    if (options.feedback) {
      showVolumeFeedback(nextVolume);
    }
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_volume", { volume: nextVolume * 100 })
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  function setPlaybackSpeed(speed: number) {
    if (!media) {
      return;
    }

    const nextSpeed = clampPlaybackSpeed(speed);
    setPlaybackSpeedValue(nextSpeed);
    anchorDisplayClock(displayTime, isPlaying, duration, nextSpeed);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_speed", { speed: nextSpeed })
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  function toggleHardwareDecoding() {
    const nextMode: HardwareDecodingMode = hardwareDecodingMode === "hardware" ? "software" : "hardware";
    setHardwareDecodingModeValue(nextMode);
    hardwareDecodingModeRef.current = nextMode;
    if (!media) {
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_hwdec", { mode: nextMode })
      .then(applyCommandSnapshot)
      .catch((error: unknown) => {
        setHardwareDecodingModeValue(hardwareDecodingMode);
        hardwareDecodingModeRef.current = hardwareDecodingMode;
        reportPlaybackError(error);
      });
  }

  function setVideoFillMode(enabled: boolean) {
    if (!media) {
      return;
    }

    const previousValue = isVideoFillEnabled;
    setIsVideoFillEnabled(enabled);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_video_fill", { enabled })
      .then(applyCommandSnapshot)
      .catch((error: unknown) => {
        setIsVideoFillEnabled(previousValue);
        reportPlaybackError(error);
      });
  }

  function setSubtitleDelay(delay: number) {
    if (!media) {
      return;
    }

    const nextDelay = clampSubtitleDelay(delay);
    setSubtitleDelayValue(nextDelay);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_subtitle_delay", { delay: nextDelay })
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  function selectTrack(kind: SelectableTrackKind, trackId: number | null) {
    if (!media) {
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_select_track", { kind, trackId })
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  async function addExternalSubtitle() {
    if (!media || isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        multiple: false,
        filters: [{ name: t.dialog.subtitle, extensions: subtitleExtensions }],
      });
      if (typeof selection !== "string") {
        return;
      }

      invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_add_subtitle", { path: selection });
      applyCommandSnapshot(snapshot);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  const displayTime = snapEndOfMediaPosition(displayPosition, duration, isPlaying);
  const progress = duration > 0 ? Math.min(100, Math.max(0, (displayTime / duration) * 100)) : 0;
  const progressRatio = progress / 100;
  const queueItems = queue.length ? queue : media ? [media] : [];
  const previousIndex = previousQueueIndex();
  const nextIndex = nextQueueIndex();
  const audioTracks = tracks.filter((track) => track.kind === "audio");
  const videoTracks = tracks.filter((track) => track.kind === "video");
  const subtitleTracks = tracks.filter((track) => track.kind === "sub");
  const isAudioOnlyMedia = Boolean(media && loadedMediaPath === media.path && isAudioMediaPath(media.path));
  const primaryAudioTrack = audioTracks.find((track) => track.selected) ?? audioTracks[0] ?? null;
  const selectedShellPreviewFormatSet = new Set(selectedShellPreviewFormats);
  const allShellPreviewFormatsSelected = shellPreviewFormats.length > 0 && selectedShellPreviewFormats.length === shellPreviewFormats.length;
  const shellPreviewVideoFormats = shellPreviewFormats.filter((format) => format.kind === "video");
  const shellPreviewAudioFormats = shellPreviewFormats.filter((format) => format.kind === "audio");
  const canShowFrames = canDisplayFrames(framesPerSecond, duration);
  const effectiveTimeDisplayMode: TimeDisplayMode = timeDisplayMode === "frames" && canShowFrames ? "frames" : "timecode";
  const totalFrames = canShowFrames ? Math.max(0, Math.floor(duration * framesPerSecond)) : 0;
  const currentFrame = canShowFrames ? Math.min(totalFrames, Math.max(0, Math.floor(displayTime * framesPerSecond))) : 0;
  const currentTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(currentFrame, locale) : formatTimecode(displayTime, duration);
  const durationTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(totalFrames, locale) : formatTimecode(duration, duration);
  const currentTimeToggleLabel = t.controls.currentTime;
  const durationTimeToggleLabel = t.controls.duration;
  const isChromeHidden = Boolean(media) && !isChromeVisible && !isChromePinned;
  const hardwareDecodingLabel = hardwareDecodingMode === "hardware" ? t.hardware.hardware : t.hardware.software;
  const hardwareDecodingToggleLabel = hardwareDecodingMode === "hardware" ? t.hardware.switchToSoftware : t.hardware.switchToHardware;
  const contextMenuItems: Array<
    | { type: "item"; id: string; label: string; icon: IconName; shortcut?: string | null; disabled?: boolean; onSelect: () => void }
    | { type: "separator"; id: string }
  > = [
    { type: "item", id: "open", label: t.contextMenu.openMedia, icon: "folder", shortcut: shortcutBindings.openMedia, disabled: isPickerOpen, onSelect: openNativeMediaFiles },
    { type: "item", id: "append-files", label: t.contextMenu.appendFiles, icon: "folderAdd", disabled: isPickerOpen, onSelect: appendNativeMediaFiles },
    { type: "item", id: "append-folder", label: t.contextMenu.appendFolder, icon: "folderAdd", disabled: isPickerOpen, onSelect: appendNativeMediaFolder },
    {
      type: "item",
      id: "play",
      label: isPlaying ? t.contextMenu.pause : media ? t.contextMenu.play : t.contextMenu.openMedia,
      icon: isPlaying ? "pause" : "play",
      shortcut: shortcutBindings.togglePlayback,
      disabled: !media && isPickerOpen,
      onSelect: togglePlayback,
    },
    { type: "item", id: "stop", label: t.contextMenu.stop, icon: "stop", disabled: !media, onSelect: stopPlayback },
    { type: "item", id: "restart", label: t.contextMenu.restart, icon: "restart", shortcut: shortcutBindings.restart, disabled: !media, onSelect: () => restartPlayback() },
    { type: "separator", id: "playback-separator" },
    { type: "item", id: "loop-mode", label: loopModeLabel(loopMode, t), icon: "restart", disabled: !media, onSelect: toggleLoopPanel },
    { type: "item", id: "media-options", label: t.contextMenu.tracksSubtitles, icon: "tracks", disabled: !media, onSelect: toggleTrackPanel },
    { type: "item", id: "playlist", label: t.contextMenu.playlist, icon: "list", shortcut: shortcutBindings.togglePlaylist, disabled: !media && queue.length === 0 && playbackHistory.length === 0, onSelect: togglePlaylist },
    { type: "item", id: "fullscreen", label: t.contextMenu.fullscreen, icon: "fullscreen", shortcut: shortcutBindings.toggleFullscreen, onSelect: toggleFullscreen },
    { type: "item", id: "settings", label: t.contextMenu.settings, icon: "settings", shortcut: shortcutBindings.openSettings, onSelect: openSettingsDialog },
    { type: "separator", id: "window-separator" },
    { type: "item", id: "close", label: t.contextMenu.closeWindow, icon: "close", onSelect: () => runWindowCommand("window_close") },
  ];

  function renderTrackList(kind: SelectableTrackKind, label: string, items: MpvTrack[]) {
    const hasSelected = items.some((track) => track.selected);

    return (
      <section className="media-panel-section">
        <header>
          <h3>{label}</h3>
          <span>{t.media.trackCount(items.length)}</span>
        </header>
        <div className="track-list">
          {kind === "subtitle" && (
            <button className={`track-item ${hasSelected ? "" : "track-item--active"}`} type="button" onClick={() => selectTrack(kind, null)}>
              <span>{t.media.closeSubtitles}</span>
              <small>{t.common.off}</small>
            </button>
          )}
          {items.map((track) => (
            <button
              key={`${track.kind}:${track.id}`}
              className={`track-item ${track.selected ? "track-item--active" : ""}`}
              type="button"
              onClick={() => selectTrack(kind, track.id)}
            >
              <span>{trackDisplayLabel(track, t)}</span>
              <small>ID {track.id}</small>
            </button>
          ))}
          {!items.length && kind !== "subtitle" && <div className="track-empty">{t.media.noSwitchableTracks}</div>}
        </div>
      </section>
    );
  }

  function renderVideoLayoutOptions() {
    if (isAudioOnlyMedia) {
      return null;
    }

    return (
      <section className="media-panel-section video-layout">
        <header>
          <h3>{t.media.videoLayout}</h3>
          <span>{isVideoFillEnabled ? t.media.videoFill : t.media.videoFit}</span>
        </header>
        <div className="video-layout-options">
          <button
            className={isVideoFillEnabled ? "video-layout-option" : "video-layout-option video-layout-option--active"}
            type="button"
            onClick={() => setVideoFillMode(false)}
          >
            <span>{t.media.videoFit}</span>
            <small>{t.media.videoFitDescription}</small>
          </button>
          <button
            className={isVideoFillEnabled ? "video-layout-option video-layout-option--active" : "video-layout-option"}
            type="button"
            onClick={() => setVideoFillMode(true)}
          >
            <span>{t.media.videoFill}</span>
            <small>{t.media.videoFillDescription}</small>
          </button>
        </div>
      </section>
    );
  }

  function renderAppearanceSettings() {
    return (
      <section className="settings-panel" aria-labelledby="appearance-settings-title">
        <div className="settings-panel-heading">
          <div>
            <h3 id="appearance-settings-title">{t.settings.appearance.title}</h3>
            <span>{activeTheme ? activeTheme.name : t.common.loading}</span>
          </div>
          <button className="settings-reset" type="button" onClick={resetAppearance}>
            {t.common.restoreDefaults}
          </button>
        </div>

        <div className="theme-grid" aria-label="Theme selection">
          {(appearanceState?.themes ?? []).map((theme) => {
            const selected = appearanceState?.activeThemeId === theme.id;
            const previewStyle = {
              "--theme-surface": theme.tokens.surface,
              "--theme-panel": theme.tokens.panelStrong,
              "--theme-text": theme.tokens.text,
              "--theme-muted": theme.tokens.muted,
              "--theme-accent": appearanceState?.accentOverride ?? theme.tokens.accent,
            } as ThemeStyleProperties;

            return (
              <button
                key={theme.id}
                className={`theme-card ${selected ? "theme-card--active" : ""}`}
                type="button"
                aria-pressed={selected}
                disabled={!theme.enabled}
                onClick={() => selectTheme(theme.id)}
              >
                <span className="theme-preview" style={previewStyle}>
                  <span />
                  <span />
                  <span />
                </span>
                <span className="theme-card-meta">
                  <strong>{theme.name}</strong>
                  <small>{theme.source === "plugin" ? t.settings.appearance.pluginTheme : t.settings.appearance.builtInTheme}</small>
                </span>
              </button>
            );
          })}
        </div>

        <section className="appearance-section" aria-labelledby="accent-settings-title">
          <header>
            <h4 id="accent-settings-title">{t.settings.appearance.accent}</h4>
            <span>{appearanceState?.accentOverride ? t.settings.appearance.custom : t.settings.appearance.followTheme}</span>
          </header>
          <label className="accent-picker">
            <span>
              <strong>{t.settings.appearance.freePick}</strong>
              <small>{hexColorForPicker(appearanceState?.accentOverride ?? activeTheme?.tokens.accent).toUpperCase()}</small>
            </span>
            <span
              className="accent-picker-preview"
              aria-hidden="true"
              style={{ "--picked-accent": hexColorForPicker(appearanceState?.accentOverride ?? activeTheme?.tokens.accent) } as ThemeStyleProperties}
            />
            <input
              type="color"
              aria-label={t.settings.appearance.freePick}
              value={hexColorForPicker(appearanceState?.accentOverride ?? activeTheme?.tokens.accent)}
              onChange={(event) => setAccentOverride(event.currentTarget.value)}
            />
          </label>
          <div className="accent-swatches" role="group" aria-label={t.settings.appearance.accent}>
            <button className={!appearanceState?.accentOverride ? "accent-default accent-swatch--active" : "accent-default"} type="button" onClick={() => setAccentOverride(null)}>
              {t.settings.appearance.themeDefault}
            </button>
            {accentSwatches.map((accent) => (
              <button
                key={accent}
                className={appearanceState?.accentOverride === accent ? "accent-swatch accent-swatch--active" : "accent-swatch"}
                type="button"
                aria-label={`${t.settings.appearance.accent} ${accent}`}
                style={{ "--swatch": accent } as ThemeStyleProperties}
                onClick={() => setAccentOverride(accent)}
              />
            ))}
          </div>
        </section>

        <section className="appearance-section" aria-labelledby="language-settings-title">
          <header>
            <h4 id="language-settings-title">{t.settings.appearance.language}</h4>
            <span>{t.settings.appearance.languageDescription}</span>
          </header>
          <div className="language-options" role="group" aria-label={t.settings.appearance.language}>
            {languageModeOptions.map((option) => (
              <button
                key={option.mode}
                className={playerPreferences.languageMode === option.mode ? "language-option language-option--active" : "language-option"}
                type="button"
                aria-pressed={playerPreferences.languageMode === option.mode}
                onClick={() => setLanguageMode(option.mode)}
              >
                {option.label[locale]}
              </button>
            ))}
          </div>
        </section>
      </section>
    );
  }

  function renderPluginSettings() {
    return (
      <section className="settings-panel" aria-labelledby="plugin-settings-title">
        <div className="settings-panel-heading">
          <div>
            <h3 id="plugin-settings-title">{t.settings.plugins.title}</h3>
            <span>{t.settings.plugins.subtitle}</span>
          </div>
          <button className="settings-reset" type="button" onClick={importThemePlugin} disabled={isPickerOpen}>
            {t.settings.plugins.importJson}
          </button>
        </div>

        <div className="plugin-list">
          {(appearanceState?.plugins ?? []).map((plugin) => (
            <div className="plugin-row" key={plugin.id}>
              <div className="plugin-meta">
                <span>{plugin.name}</span>
                <small>
                  {plugin.id} · {t.settings.plugins.themeCount(plugin.themeCount)} · v{plugin.version}
                </small>
                {plugin.description && <p>{plugin.description}</p>}
              </div>
              <label className="plugin-toggle">
                <input type="checkbox" checked={plugin.enabled} onChange={(event) => setThemePluginEnabled(plugin.id, event.currentTarget.checked)} />
                <span>{plugin.enabled ? t.settings.plugins.enabled : t.settings.plugins.disabled}</span>
              </label>
            </div>
          ))}
          {!appearanceState?.plugins.length && <div className="plugin-empty">{t.settings.plugins.empty}</div>}
        </div>
      </section>
    );
  }

  function renderPlaybackSettings() {
    return (
      <section className="settings-panel" aria-labelledby="playback-settings-title">
        <div className="settings-panel-heading">
          <div>
            <h3 id="playback-settings-title">{t.settings.playback.title}</h3>
            <span>{t.settings.playback.subtitle}</span>
          </div>
          <button className="settings-reset" type="button" onClick={clearPlaybackHistory} disabled={!playbackHistory.length}>
            {t.settings.playback.clearHistory}
          </button>
        </div>

        <div className="preference-list">
          <label className="preference-row">
            <span>
              <strong>{t.settings.playback.incognito}</strong>
              <small>{t.settings.playback.incognitoDescription}</small>
            </span>
            <input type="checkbox" checked={playerPreferences.incognitoMode} onChange={(event) => setIncognitoMode(event.currentTarget.checked)} />
            <span className="preference-switch" aria-hidden="true">
              <span />
            </span>
          </label>

          <label className="preference-row">
            <span>
              <strong>{t.settings.playback.quietKeyboard}</strong>
              <small>{t.settings.playback.quietKeyboardDescription}</small>
            </span>
            <input type="checkbox" checked={playerPreferences.quietKeyboardControls} onChange={(event) => setQuietKeyboardControls(event.currentTarget.checked)} />
            <span className="preference-switch" aria-hidden="true">
              <span />
            </span>
          </label>
        </div>

        <section className="shell-preview-card" aria-label={t.settings.shellPreview.title}>
          <header className="shell-preview-card-header">
            <span className="shell-preview-card-icon" aria-hidden="true">
              <Icon name="preview" />
            </span>
            <span className="shell-preview-card-copy">
              <strong>{t.settings.shellPreview.title}</strong>
              <small>{t.settings.shellPreview.description}</small>
              {shellPreviewRegistrationStatus && <small className="shell-preview-status">{shellPreviewRegistrationStatus}</small>}
            </span>
            <span className="shell-preview-actions">
              <button className="shell-preview-action" type="button" onClick={toggleAllShellPreviewFormats} disabled={!shellPreviewFormats.length}>
                {allShellPreviewFormatsSelected ? t.settings.shellPreview.clearAll : t.settings.shellPreview.selectAll}
              </button>
              <button className="shell-preview-action" type="button" onClick={resetShellPreviewFormatsToDefault} disabled={!shellPreviewFormats.length}>
                {t.settings.shellPreview.defaults}
              </button>
              <button className="shell-preview-action" type="button" onClick={openDefaultAppsSettings}>
                {t.settings.shellPreview.defaultApps}
              </button>
              <button className="shell-preview-action" type="button" onClick={registerShellPreviews} disabled={isRegisteringShellPreview || selectedShellPreviewFormats.length === 0}>
                {isRegisteringShellPreview ? t.settings.shellPreview.registering : t.settings.shellPreview.register(selectedShellPreviewFormats.length)}
              </button>
            </span>
          </header>

          <div className="shell-preview-format-groups">
            {[
              { kind: "video", label: t.settings.shellPreview.video, formats: shellPreviewVideoFormats },
              { kind: "audio", label: t.settings.shellPreview.audio, formats: shellPreviewAudioFormats },
            ].map((group) => (
              <section className="shell-preview-format-group" key={group.kind} aria-label={`${group.label} preview formats`}>
                <header>
                  <strong>{group.label}</strong>
                  <small>{group.formats.filter((format) => selectedShellPreviewFormatSet.has(format.extension)).length}/{group.formats.length}</small>
                </header>
                <div className="shell-preview-format-grid">
                  {group.formats.map((format) => (
                    <button
                      key={format.extension}
                      className={
                        selectedShellPreviewFormatSet.has(format.extension)
                          ? "shell-preview-format shell-preview-format--selected"
                          : "shell-preview-format shell-preview-format--unselected"
                      }
                      type="button"
                      aria-pressed={selectedShellPreviewFormatSet.has(format.extension)}
                      title={format.mime}
                      onClick={() => toggleShellPreviewFormat(format.extension)}
                    >
                      <span>{format.extension}</span>
                    </button>
                  ))}
                </div>
              </section>
            ))}
          </div>
        </section>
      </section>
    );
  }

  function renderShortcutSettings() {
    return (
      <section className="settings-panel" aria-labelledby="shortcut-settings-title">
        <div className="settings-panel-heading">
          <div>
            <h3 id="shortcut-settings-title">{t.settings.shortcuts.title}</h3>
            <span>{recordingShortcutAction ? t.common.inputting : t.settings.shortcuts.subtitle}</span>
          </div>
          <button className="settings-reset" type="button" onClick={resetShortcutBindings}>
            {t.common.restoreDefaults}
          </button>
        </div>

        <div className="shortcut-list">
          {shortcutDefinitions.map((definition) => {
            const isRecording = recordingShortcutAction === definition.action;
            const binding = shortcutBindings[definition.action];

            return (
              <div className="shortcut-row" key={definition.action}>
                <div className="shortcut-meta">
                  <span>{definition.label}</span>
                  <small>{definition.group}</small>
                </div>
                <div className="shortcut-editor">
                  <button
                    className={`shortcut-capture ${isRecording ? "shortcut-capture--recording" : ""}`}
                    type="button"
                    aria-pressed={isRecording}
                    onClick={() => setRecordingShortcutAction(definition.action)}
                  >
                    <kbd>{isRecording ? t.common.inputting : formatShortcutChord(binding, t)}</kbd>
                  </button>
                  <button
                    className="shortcut-clear"
                    type="button"
                    aria-label={`Clear shortcut for ${definition.label}`}
                    disabled={!binding}
                    onClick={() => assignShortcut(definition.action, null)}
                  >
                    <Icon name="close" />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </section>
    );
  }

  return (
    <main
      className="app-shell"
      style={appearanceStyle}
      onContextMenu={openContextMenu}
      onKeyDown={recordUserActivity}
      onPointerDown={handleShellPointerDown}
      onPointerLeave={handleShellPointerLeave}
      onPointerMove={recordUserActivity}
      onWheel={handleShellWheel}
    >
      <section className={`window-shell ${media ? "window-shell--loaded" : ""}`} aria-label="OpenPlayer">
        <section className={`stage ${media ? "stage--loaded" : ""} ${isChromeHidden ? "stage--chrome-hidden" : ""}`} aria-label="Player surface">
          {!media && (
            <div className="empty-open">
              <img className="empty-open-logo" src={openPlayerLogoUrl} alt="" draggable={false} />
              <span>{t.contextMenu.openMedia}</span>
              {platformSupport && !platformSupport.mpvEmbedVideo && <small className="platform-support-note">{platformUnsupportedPlaybackMessage(platformSupport, t)}</small>}
            </div>
          )}

          {isAudioOnlyMedia && media && (
            <div className={isPlaying ? "audio-visualizer" : "audio-visualizer audio-visualizer--paused"} aria-hidden="true">
              <div className="audio-visualizer-bars">
                {audioVisualizerBarLevels.map((level, index) => (
                  <span
                    key={index}
                    style={{ "--bar-level": String(level), "--bar-delay": `${index * -86}ms` } as ThemeStyleProperties}
                  />
                ))}
              </div>
              <div className="audio-visualizer-grid">
                <div className="audio-visualizer-copy">
                  <span>{media.name}</span>
                  <small>
                    {(primaryAudioTrack?.codec ?? "audio").toUpperCase()} · {formatTimecode(displayTime, duration)}
                  </small>
                </div>
              </div>
            </div>
          )}

          <div
            className="drag-region"
            aria-hidden="true"
            onAuxClick={(event) => event.preventDefault()}
            onDoubleClick={handleDragRegionDoubleClick}
            onPointerDown={handleDragRegionPointerDown}
            onPointerMove={handleDragRegionPointerMove}
            onPointerUp={handleDragRegionPointerEnd}
            onPointerCancel={handleDragRegionPointerEnd}
          />

          {resizeRegions.map((region) => (
            <div
              key={region.direction}
              aria-hidden="true"
              className={`resize-region ${region.className}`}
              onPointerEnter={(event) => handleResizePointerEnter(event, region.direction)}
              onPointerLeave={handleResizePointerLeave}
              onPointerDown={(event) => handleResizePointerDown(event, region.direction)}
              onPointerMove={handleResizePointerMove}
              onPointerUp={handleResizePointerEnd}
              onPointerCancel={handleResizePointerEnd}
            />
          ))}

          {resizeFeedback && (
            <div
              aria-hidden="true"
              className={`resize-feedback resize-feedback--${resizeDirectionClassName(resizeFeedback.direction)} ${resizeFeedback.active ? "resize-feedback--active" : ""}`}
            >
              <span className="resize-feedback-line resize-feedback-line--north" />
              <span className="resize-feedback-line resize-feedback-line--south" />
              <span className="resize-feedback-line resize-feedback-line--east" />
              <span className="resize-feedback-line resize-feedback-line--west" />
              <span className="resize-feedback-corner resize-feedback-corner--north-east" />
              <span className="resize-feedback-corner resize-feedback-corner--north-west" />
              <span className="resize-feedback-corner resize-feedback-corner--south-east" />
              <span className="resize-feedback-corner resize-feedback-corner--south-west" />
            </div>
          )}

          <div className="window-controls" aria-label={t.contextMenu.closeWindow}>
            <button type="button" aria-label={t.controls.minimize} onClick={() => runWindowCommand("window_minimize")}>
              <Icon name="minimize" />
            </button>
            <button type="button" aria-label={t.controls.maximize} onClick={() => runWindowCommand("window_toggle_maximize")}>
              <Icon name="maximize" />
            </button>
            <button className="window-control-close" type="button" aria-label={t.controls.close} onClick={() => runWindowCommand("window_close")}>
              <Icon name="close" />
            </button>
          </div>

          {playbackError && <div className="playback-error" role="alert">{playbackError}</div>}
          {volumeFeedback && (
            <div className="volume-feedback" role="status" aria-live="polite">
              <Icon name="volume" />
              <span>{Math.round(volumeFeedback.level * 100)}%</span>
            </div>
          )}

          <div className="transport" aria-label={t.contextMenu.play}>
            <div className="transport-row">
              <button
                className="transport-time transport-time--toggle"
                type="button"
                aria-label={currentTimeToggleLabel}
                aria-pressed={effectiveTimeDisplayMode === "frames"}
                onClick={toggleTimeDisplayMode}
                disabled={!canShowFrames}
              >
                {currentTransportLabel}
              </button>
              <button className="timeline-step-button" type="button" aria-label={t.controls.previousVideo} onClick={playPreviousQueueItem} disabled={previousIndex === null}>
                <Icon name="previous" />
              </button>
              <div className="seek-control" style={{ "--progress": `${progress}%`, "--progress-ratio": progressRatio } as CSSProperties}>
                <div className="seek-rail" aria-hidden="true">
                  <div className="seek-progress" />
                </div>
                <div className="seek-thumb" aria-hidden="true" />
                <input
                  className="seek-slider"
                  type="range"
                  min="0"
                  max={duration || 0}
                  step="any"
                  value={displayTime}
                  aria-label={t.controls.seek}
                  onChange={(event) => seekTo(Number(event.currentTarget.value))}
                  onPointerUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                  onKeyUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                  onBlur={(event) => commitSeekTo(Number(event.currentTarget.value))}
                  disabled={!media || duration <= 0}
                />
              </div>
              <button className="timeline-step-button" type="button" aria-label={t.controls.nextVideo} onClick={playNextQueueItem} disabled={nextIndex === null}>
                <Icon name="next" />
              </button>
              <button
                className="transport-time transport-time--toggle"
                type="button"
                aria-label={durationTimeToggleLabel}
                aria-pressed={effectiveTimeDisplayMode === "frames"}
                onClick={toggleTimeDisplayMode}
                disabled={!canShowFrames}
              >
                {durationTransportLabel}
              </button>
            </div>

            <div className="control-strip">
              <button className="open-media-button" type="button" aria-label={t.controls.openMedia} onClick={openNativeMediaFiles} disabled={isPickerOpen}>
                <Icon name="folder" />
              </button>
              <button type="button" aria-label={t.controls.stop} onClick={stopPlayback} disabled={!media}>
                <Icon name="stop" />
              </button>
              <button className="control-primary" type="button" aria-label={isPlaying ? t.controls.pause : media ? t.controls.play : t.controls.openMedia} onClick={togglePlayback} disabled={!media && isPickerOpen}>
                <Icon name={isPlaying ? "pause" : "play"} />
              </button>
              <button className={mediaPanelMode === "loop" ? "loop-toggle loop-toggle--open" : "loop-toggle"} type="button" aria-label={t.controls.openLoopMode} aria-expanded={mediaPanelMode === "loop"} onClick={toggleLoopPanel} disabled={!media}>
                <Icon name="restart" />
              </button>
              <label className="volume-control" aria-label={t.controls.volume}>
                <Icon name="volume" />
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={volumeLevel}
                  aria-label={t.controls.volume}
                  onChange={(event) => setVolume(Number(event.currentTarget.value))}
                />
              </label>
              <button className="speed-toggle" type="button" aria-label={t.controls.openPlaybackSpeed} aria-expanded={mediaPanelMode === "speed"} onClick={toggleSpeedPanel} disabled={!media}>
                {formatPlaybackSpeed(playbackSpeed)}
              </button>
              <button
                className={`tracks-toggle ${mediaPanelMode === "tracks" ? "tracks-toggle--open" : ""}`}
                type="button"
                aria-label={t.controls.openTracks}
                aria-expanded={mediaPanelMode === "tracks"}
                onClick={toggleTrackPanel}
                disabled={!media}
              >
                <Icon name="tracks" />
              </button>
              <button
                className={`playlist-toggle ${isPlaylistOpen ? "playlist-toggle--open" : ""}`}
                type="button"
                aria-label={t.controls.togglePlaylist}
                aria-expanded={isPlaylistOpen}
                onClick={togglePlaylist}
              >
                <Icon name="list" />
              </button>
              <button
                className={`decode-toggle decode-toggle--${hardwareDecodingMode}`}
                type="button"
                aria-label={hardwareDecodingToggleLabel}
                aria-pressed={hardwareDecodingMode === "hardware"}
                title={hardwareDecodingToggleLabel}
                onClick={toggleHardwareDecoding}
              >
                <Icon name="cpu" />
                <span>{hardwareDecodingLabel}</span>
              </button>
            </div>
          </div>

          {mediaPanelMode === "speed" && media && (
            <aside
              className="media-panel media-panel--speed"
              aria-label={t.media.speed}
              onContextMenu={(event) => event.stopPropagation()}
              onPointerDown={(event) => event.stopPropagation()}
            >
              <section className="media-panel-section">
                <header>
                  <h3>{t.media.speed}</h3>
                  <span>{formatPlaybackSpeed(playbackSpeed)}</span>
                </header>
                <div className="speed-options" role="group" aria-label={t.media.speed}>
                  {playbackSpeedOptions.map((speed) => (
                    <button
                      key={speed}
                      className={Math.abs(playbackSpeed - speed) < 0.001 ? "speed-option speed-option--active" : "speed-option"}
                      type="button"
                      aria-pressed={Math.abs(playbackSpeed - speed) < 0.001}
                      onClick={() => setPlaybackSpeed(speed)}
                    >
                      {formatPlaybackSpeed(speed)}
                    </button>
                  ))}
                </div>
              </section>
            </aside>
          )}

          {mediaPanelMode === "loop" && media && (
            <aside
              className="media-panel media-panel--loop"
              aria-label={t.media.loopMode}
              onContextMenu={(event) => event.stopPropagation()}
              onPointerDown={(event) => event.stopPropagation()}
            >
              <section className="media-panel-section">
                <header>
                  <h3>{t.media.loopMode}</h3>
                  <span>{loopModeLabel(loopMode, t)}</span>
                </header>
                <div className="loop-options" role="group" aria-label={t.media.loopMode}>
                  {loopModeOptions.map((option) => (
                    <button
                      key={option.mode}
                      className={loopMode === option.mode ? "loop-option loop-option--active" : "loop-option"}
                      type="button"
                      aria-pressed={loopMode === option.mode}
                      onClick={() => setLoopMode(option.mode)}
                    >
                      <span>{option.label}</span>
                      <small>{option.description}</small>
                    </button>
                  ))}
                </div>
              </section>
            </aside>
          )}

          {mediaPanelMode === "tracks" && media && (
            <aside
              className="media-panel media-panel--tracks"
              aria-label={t.contextMenu.tracksSubtitles}
              onContextMenu={(event) => event.stopPropagation()}
              onPointerDown={(event) => event.stopPropagation()}
            >
              {renderVideoLayoutOptions()}
              {renderTrackList("audio", t.media.audioTracks, audioTracks)}
              {renderTrackList("video", t.media.videoTracks, videoTracks)}
              {renderTrackList("subtitle", t.media.subtitles, subtitleTracks)}

              <section className="media-panel-section subtitle-delay">
                <header>
                  <h3>{t.media.subtitleSync}</h3>
                  <span>{formatSubtitleDelay(subtitleDelay)}</span>
                </header>
                <div className="subtitle-delay-controls">
                  <button type="button" onClick={() => setSubtitleDelay(subtitleDelay - SUBTITLE_DELAY_STEP_SECONDS)}>
                    -0.1s
                  </button>
                  <output>{formatSubtitleDelay(subtitleDelay)}</output>
                  <button type="button" onClick={() => setSubtitleDelay(subtitleDelay + SUBTITLE_DELAY_STEP_SECONDS)}>
                    +0.1s
                  </button>
                  <button type="button" onClick={() => setSubtitleDelay(0)} disabled={Math.abs(subtitleDelay) < 0.005}>
                    {t.common.reset}
                  </button>
                </div>
              </section>

              <button className="subtitle-load" type="button" onClick={addExternalSubtitle} disabled={isPickerOpen}>
                {t.media.loadExternalSubtitle}
              </button>
            </aside>
          )}

          {isPlaylistOpen && (
            <aside className="playlist-drawer playlist-drawer--open" aria-label={t.media.playlist}>
              <header className="playlist-drawer-header">
                <h3>{t.media.playlist}</h3>
                <div className="playlist-actions">
                  <button type="button" onClick={appendNativeMediaFiles} disabled={isPickerOpen}>
                    <Icon name="folderAdd" />
                    <span>{t.media.addFiles}</span>
                  </button>
                  <button type="button" onClick={appendNativeMediaFolder} disabled={isPickerOpen}>
                    <Icon name="folder" />
                    <span>{t.media.addFolder}</span>
                  </button>
                </div>
              </header>

              {queueItems.length > 0 && (
                <ol>
                  {queueItems.map((item, index) => (
                    <li key={item.id}>
                      <button
                        className={`playlist-item ${index === currentIndex ? "playlist-item--active" : ""}`}
                        type="button"
                        aria-current={index === currentIndex ? "true" : undefined}
                        onClick={() => chooseQueueItem(index)}
                      >
                        <span>{item.name}</span>
                      </button>
                    </li>
                  ))}
                </ol>
              )}

              {playbackHistory.length > 0 && (
                <section className="history-section" aria-label={t.media.recent}>
                  <header>
                    <h3>{t.media.recent}</h3>
                    <button className="history-clear" type="button" onClick={clearPlaybackHistory}>
                      {t.common.clear}
                    </button>
                  </header>
                  <div className="history-list">
                    {playbackHistory.map((entry) => (
                      <button
                        key={entry.path}
                        className={`history-item ${media?.path === entry.path ? "history-item--active" : ""}`}
                        type="button"
                        title={entry.path}
                        onClick={() => openHistoryEntry(entry)}
                      >
                        <span>{entry.name}</span>
                        <small>{formatHistoryProgress(entry, t)}</small>
                      </button>
                    ))}
                  </div>
                </section>
              )}

              {queueItems.length === 0 && playbackHistory.length === 0 && <div className="playlist-empty">{t.media.emptyPlaylist}</div>}
            </aside>
          )}

          {contextMenu && (
            <div
              className="context-menu"
              role="menu"
              aria-label={t.contextMenu.settings}
              style={{ left: contextMenu.x, top: contextMenu.y }}
              onContextMenu={(event) => {
                event.preventDefault();
                event.stopPropagation();
              }}
              onPointerDown={(event) => event.stopPropagation()}
            >
              {contextMenuItems.map((item) =>
                item.type === "separator" ? (
                  <div key={item.id} className="context-menu-separator" role="separator" />
                ) : (
                  <button
                    key={item.id}
                    className="context-menu-item"
                    type="button"
                    role="menuitem"
                    disabled={item.disabled}
                    onClick={() => {
                      setContextMenu(null);
                      item.onSelect();
                    }}
                  >
                    <Icon name={item.icon} />
                    <span>{item.label}</span>
                    {item.shortcut && <kbd>{formatShortcutChord(item.shortcut, t)}</kbd>}
                  </button>
                ),
              )}
            </div>
          )}

          {isSettingsOpen && (
            <div
              className="settings-backdrop"
              onPointerDown={(event) => {
                if (event.target === event.currentTarget) {
                  closeSettingsDialog();
                }
              }}
            >
              <section
                ref={settingsDialogRef}
                className="settings-dialog"
                role="dialog"
                aria-modal="true"
                aria-labelledby="settings-title"
                tabIndex={-1}
                onContextMenu={(event) => event.stopPropagation()}
                onPointerDown={(event) => event.stopPropagation()}
              >
                <header className="settings-header">
                  <div>
                    <span className="settings-kicker">OpenPlayer</span>
                    <h2 id="settings-title">{t.settings.title}</h2>
                  </div>
                  <button className="settings-close" type="button" aria-label={t.controls.close} onClick={closeSettingsDialog}>
                    <Icon name="close" />
                  </button>
                </header>

                <div className="settings-layout">
                  <nav className="settings-nav" aria-label={t.settings.title}>
                    <button
                      className={`settings-nav-item ${settingsSection === "appearance" ? "settings-nav-item--active" : ""}`}
                      type="button"
                      aria-current={settingsSection === "appearance" ? "page" : undefined}
                      onClick={() => setSettingsSection("appearance")}
                    >
                      <Icon name="palette" />
                      <span>{t.settings.nav.appearance}</span>
                    </button>
                    <button
                      className={`settings-nav-item ${settingsSection === "plugins" ? "settings-nav-item--active" : ""}`}
                      type="button"
                      aria-current={settingsSection === "plugins" ? "page" : undefined}
                      onClick={() => setSettingsSection("plugins")}
                    >
                      <Icon name="plugin" />
                      <span>{t.settings.nav.plugins}</span>
                    </button>
                    <button
                      className={`settings-nav-item ${settingsSection === "playback" ? "settings-nav-item--active" : ""}`}
                      type="button"
                      aria-current={settingsSection === "playback" ? "page" : undefined}
                      onClick={() => setSettingsSection("playback")}
                    >
                      <Icon name="play" />
                      <span>{t.settings.nav.playback}</span>
                    </button>
                    <button
                      className={`settings-nav-item ${settingsSection === "shortcuts" ? "settings-nav-item--active" : ""}`}
                      type="button"
                      aria-current={settingsSection === "shortcuts" ? "page" : undefined}
                      onClick={() => setSettingsSection("shortcuts")}
                    >
                      <Icon name="settings" />
                      <span>{t.settings.nav.shortcuts}</span>
                    </button>
                  </nav>

                  {settingsSection === "appearance" && renderAppearanceSettings()}
                  {settingsSection === "plugins" && renderPluginSettings()}
                  {settingsSection === "playback" && renderPlaybackSettings()}
                  {settingsSection === "shortcuts" && renderShortcutSettings()}
                </div>
              </section>
            </div>
          )}
        </section>
      </section>
    </main>
  );
}

export default App;
