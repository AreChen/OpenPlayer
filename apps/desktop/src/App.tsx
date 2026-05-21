import { useEffect, useRef, useState, type CSSProperties, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

type MediaItem = {
  id: string;
  name: string;
  path: string;
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
  volume: number;
  tracks: MpvTrack[];
};

type PendingSeek = {
  target: number;
  startedAt: number;
};

type PlaybackClockAnchor = {
  position: number;
  startedAt: number;
  playing: boolean;
  speed: number;
};

type TimeDisplayMode = "timecode" | "frames";
type SelectableTrackKind = "audio" | "video" | "subtitle";

type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";

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
type IconName = "close" | "folder" | "fullscreen" | "list" | "maximize" | "minimize" | "pause" | "play" | "restart" | "settings" | "volume";

const playableExtensions = ["3gp", "aac", "avi", "flac", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "oga", "ogg", "ogv", "opus", "wav", "webm"];
const subtitleExtensions = ["ass", "srt", "ssa", "sub", "vtt"];
const playbackSpeedOptions = [0.5, 0.75, 1, 1.25, 1.5, 2];
const OPENPLAYER_SHORTCUTS_STORAGE_KEY = "openplayer.shortcuts.v3";
const SEEK_CONFIRM_TOLERANCE_SECONDS = 0.75;
const SEEK_SNAPSHOT_SUPPRESS_MS = 1600;
const AUTO_HIDE_CONTROLS_MS = 5000;
const END_OF_MEDIA_SNAP_TOLERANCE_SECONDS = 0.5;
const CONTEXT_MENU_WIDTH = 236;
const CONTEXT_MENU_HEIGHT = 336;
const DEFAULT_SEEK_STEP_SECONDS = 5;
const DEFAULT_VOLUME_STEP = 0.05;
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
const shortcutDefinitions: ShortcutDefinition[] = [
  { action: "openMedia", label: "打开媒体", group: "文件" },
  { action: "togglePlayback", label: "播放 / 暂停", group: "播放" },
  { action: "restart", label: "从头播放", group: "播放" },
  { action: "togglePlaylist", label: "播放列表", group: "播放" },
  { action: "seekBackward", label: "后退 5 秒", group: "定位" },
  { action: "seekForward", label: "前进 5 秒", group: "定位" },
  { action: "frameBackward", label: "上一帧", group: "逐帧" },
  { action: "frameForward", label: "下一帧", group: "逐帧" },
  { action: "volumeDown", label: "降低音量", group: "音量" },
  { action: "volumeUp", label: "提高音量", group: "音量" },
  { action: "toggleFullscreen", label: "全屏", group: "窗口" },
  { action: "openSettings", label: "设置", group: "窗口" },
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

function formatShortcutChord(chord: string | null) {
  if (!chord) {
    return "未设置";
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

function readShortcutBindings() {
  try {
    const stored = window.localStorage.getItem(OPENPLAYER_SHORTCUTS_STORAGE_KEY);
    if (!stored) {
      return defaultShortcutBindings;
    }

    const parsed = JSON.parse(stored) as Partial<Record<ShortcutAction, unknown>>;
    const merged: ShortcutBindings = { ...defaultShortcutBindings };
    for (const definition of shortcutDefinitions) {
      const value = parsed[definition.action];
      if (typeof value === "string" || value === null) {
        merged[definition.action] = value;
      }
    }

    return merged;
  } catch {
    return defaultShortcutBindings;
  }
}

function isShortcutAction(value: unknown): value is ShortcutAction {
  return typeof value === "string" && shortcutDefinitions.some((definition) => definition.action === value);
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

function startMainWindowResize(direction: ResizeDirection) {
  invoke("window_start_resize", { direction }).catch((error: unknown) => {
    console.error(`Window resize failed: ${direction}`, error);
  });
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

function formatFrameCount(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0";
  }

  return Math.floor(value).toLocaleString("en-US");
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

function trackDisplayLabel(track: MpvTrack) {
  const title = track.title || `${track.kind.toUpperCase()} ${track.id}`;
  const details = [track.language?.toUpperCase(), track.codec, track.external ? "外部" : null].filter(Boolean);
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

function mediaItemFromPath(path: string): MediaItem {
  const normalized = path.replace(/\\/g, "/");
  return {
    id: nextMediaItemId(),
    name: normalized.split("/").pop() || path,
    path,
  };
}

function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    close: "M6 6l12 12M18 6 6 18",
    folder: "M3 7.5h6l2 2h10v8.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5Z",
    fullscreen: "M8 4H4v4M16 4h4v4M20 16v4h-4M8 20H4v-4",
    list: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
    maximize: "M7 7h10v10H7z",
    minimize: "M6 12h12",
    pause: "M8 6h3v12H8zM13 6h3v12h-3z",
    play: "M8 5v14l11-7z",
    restart: "M5 12a7 7 0 1 0 2-4.9M5 5v5h5",
    settings: "M12 8.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7ZM19 12a7.2 7.2 0 0 0-.08-1l2-1.55-2-3.45-2.36.95a7.4 7.4 0 0 0-1.72-1L14.5 3h-4l-.34 2.95a7.4 7.4 0 0 0-1.72 1L6.08 6l-2 3.45L6.08 11A7.2 7.2 0 0 0 6 12c0 .34.03.67.08 1l-2 1.55 2 3.45 2.36-.95c.53.42 1.1.75 1.72 1l.34 2.95h4l.34-2.95c.62-.25 1.19-.58 1.72-1l2.36.95 2-3.45-2-1.55c.05-.33.08-.66.08-1Z",
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
  const [isPlaylistOpen, setIsPlaylistOpen] = useState(false);
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [displayPosition, setDisplayPosition] = useState(0);
  const [volumeLevel, setVolumeLevel] = useState(0.82);
  const [playbackSpeed, setPlaybackSpeedValue] = useState(1);
  const [tracks, setTracks] = useState<MpvTrack[]>([]);
  const [framesPerSecond, setFramesPerSecond] = useState(0);
  const [timeDisplayMode, setTimeDisplayMode] = useState<TimeDisplayMode>("timecode");
  const [isPlaying, setIsPlaying] = useState(false);
  const [isChromeVisible, setIsChromeVisible] = useState(true);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<ContextMenuPosition | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isMediaPanelOpen, setIsMediaPanelOpen] = useState(false);
  const [shortcutBindings, setShortcutBindings] = useState<ShortcutBindings>(readShortcutBindings);
  const [recordingShortcutAction, setRecordingShortcutAction] = useState<ShortcutAction | null>(null);
  const pendingSeekRef = useRef<PendingSeek | null>(null);
  const playbackClockAnchorRef = useRef<PlaybackClockAnchor>({ position: 0, startedAt: performance.now(), playing: false, speed: 1 });
  const snapshotRequestIdRef = useRef(0);
  const chromeHideTimerRef = useRef<number | null>(null);
  const shortcutKeyDownRef = useRef<(event: KeyboardEvent) => void>(() => undefined);
  const nativeShortcutActionRef = useRef<(action: ShortcutAction) => void>(() => undefined);
  const settingsDialogRef = useRef<HTMLElement | null>(null);
  const media = currentIndex === null ? null : (queue[currentIndex] ?? null);
  const isChromePinned = !media || isPlaylistOpen || isMediaPanelOpen || isPickerOpen || playbackError !== null || contextMenu !== null || isSettingsOpen;

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
    setIsChromeVisible(true);
    scheduleChromeHide();
    return clearChromeHideTimer;
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

  shortcutKeyDownRef.current = (event: KeyboardEvent) => {
    recordUserActivity();

    if (recordingShortcutAction) {
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
    performShortcutAction(shortcut.action);
  };

  nativeShortcutActionRef.current = (action: ShortcutAction) => {
    recordUserActivity();
    if (contextMenu || isSettingsOpen || recordingShortcutAction) {
      return;
    }

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

  function clearChromeHideTimer() {
    if (chromeHideTimerRef.current !== null) {
      window.clearTimeout(chromeHideTimerRef.current);
      chromeHideTimerRef.current = null;
    }
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

  function invalidatePendingSnapshots() {
    snapshotRequestIdRef.current += 1;
  }

  function applyCommandSnapshot(snapshot: MpvSnapshot) {
    invalidatePendingSnapshots();
    applySnapshot(snapshot);
  }

  function applySnapshot(snapshot: MpvSnapshot) {
    const snapshotPosition = Number.isFinite(snapshot.position) ? snapshot.position : 0;
    const snapshotDuration = Number.isFinite(snapshot.duration) ? snapshot.duration : 0;
    const snapshotSpeed = clampPlaybackSpeed(snapshot.speed);
    const pendingSeek = pendingSeekRef.current;
    const nextIsPlaying = !snapshot.paused && snapshot.status === "playing";

    setDuration(snapshotDuration);
    setIsPlaying(nextIsPlaying);
    setFramesPerSecond(Number.isFinite(snapshot.fps) && snapshot.fps > 0 ? snapshot.fps : 0);
    setPlaybackSpeedValue(snapshotSpeed);
    setTracks(Array.isArray(snapshot.tracks) ? snapshot.tracks : []);
    setVolumeLevel(Math.min(1, Math.max(0, snapshot.volume / 100)));

    if (pendingSeek) {
      const isConfirmed = Math.abs(snapshotPosition - pendingSeek.target) <= SEEK_CONFIRM_TOLERANCE_SECONDS;
      const isExpired = performance.now() - pendingSeek.startedAt > SEEK_SNAPSHOT_SUPPRESS_MS;
      if (!isConfirmed && !isExpired) {
        return;
      }

      pendingSeekRef.current = null;
    }

    setCurrentTime(snapshotPosition);
    anchorDisplayClock(snapshotPosition, nextIsPlaying, snapshotDuration, snapshotSpeed);
  }

  function reportPlaybackError(error: unknown) {
    setPlaybackError(error instanceof Error ? error.message : String(error));
  }

  function openMpvPath(path: string) {
    invalidatePendingSnapshots();
    return invoke<MpvSnapshot>("mpv_overlay_open_path", { path }).then((snapshot) => {
      pendingSeekRef.current = null;
      setPlaybackError(null);
      applyCommandSnapshot(snapshot);
    });
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

  function openSettingsDialog() {
    setContextMenu(null);
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
    if (media && !isChromePinned) {
      setIsChromeVisible(false);
    }
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
          commitSeekTo(0);
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
        setVolume(volumeLevel - DEFAULT_VOLUME_STEP);
        break;
      case "volumeUp":
        setVolume(volumeLevel + DEFAULT_VOLUME_STEP);
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

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: "Media", extensions: playableExtensions }],
      });
      const paths = typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
      const nextQueue = paths.map(mediaItemFromPath);
      if (!nextQueue.length) {
        return;
      }

      setQueue(nextQueue);
      setCurrentIndex(0);
      setIsPlaylistOpen(nextQueue.length > 1);
      await openMpvPath(nextQueue[0].path);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  function chooseQueueItem(index: number) {
    const item = queue[index];
    if (!item || index === currentIndex) {
      return;
    }

    setCurrentIndex(index);
    openMpvPath(item.path).catch(reportPlaybackError);
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
      startMainWindowDrag();
    }
  }

  function handleResizePointerDown(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    recordUserActivity();
    startMainWindowResize(direction);
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
    setIsPlaylistOpen((isOpen) => !isOpen);
  }

  function toggleMediaPanel() {
    setIsMediaPanelOpen((isOpen) => !isOpen);
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

  function setVolume(value: number) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
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
        filters: [{ name: "Subtitles", extensions: subtitleExtensions }],
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
  const audioTracks = tracks.filter((track) => track.kind === "audio");
  const videoTracks = tracks.filter((track) => track.kind === "video");
  const subtitleTracks = tracks.filter((track) => track.kind === "sub");
  const canShowFrames = canDisplayFrames(framesPerSecond, duration);
  const effectiveTimeDisplayMode: TimeDisplayMode = timeDisplayMode === "frames" && canShowFrames ? "frames" : "timecode";
  const totalFrames = canShowFrames ? Math.max(0, Math.floor(duration * framesPerSecond)) : 0;
  const currentFrame = canShowFrames ? Math.min(totalFrames, Math.max(0, Math.floor(displayTime * framesPerSecond))) : 0;
  const currentTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(currentFrame) : formatTimecode(displayTime, duration);
  const durationTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(totalFrames) : formatTimecode(duration, duration);
  const currentTimeToggleLabel = canShowFrames ? "Toggle current playback time and frame display" : "Current playback time; frame display unavailable for this media";
  const durationTimeToggleLabel = canShowFrames ? "Toggle total duration and frame display" : "Total duration; frame display unavailable for this media";
  const isChromeHidden = Boolean(media) && !isChromeVisible && !isChromePinned;
  const contextMenuItems: Array<
    | { type: "item"; id: string; label: string; icon: IconName; shortcut?: string | null; disabled?: boolean; onSelect: () => void }
    | { type: "separator"; id: string }
  > = [
    { type: "item", id: "open", label: "打开媒体", icon: "folder", shortcut: shortcutBindings.openMedia, disabled: isPickerOpen, onSelect: openNativeMediaFiles },
    {
      type: "item",
      id: "play",
      label: isPlaying ? "暂停" : media ? "播放" : "打开媒体",
      icon: isPlaying ? "pause" : "play",
      shortcut: shortcutBindings.togglePlayback,
      disabled: !media && isPickerOpen,
      onSelect: togglePlayback,
    },
    { type: "item", id: "restart", label: "从头播放", icon: "restart", shortcut: shortcutBindings.restart, disabled: !media, onSelect: () => commitSeekTo(0) },
    { type: "separator", id: "playback-separator" },
    { type: "item", id: "media-options", label: "播放选项", icon: "settings", disabled: !media, onSelect: toggleMediaPanel },
    { type: "item", id: "playlist", label: "播放列表", icon: "list", shortcut: shortcutBindings.togglePlaylist, disabled: !media && queue.length === 0, onSelect: togglePlaylist },
    { type: "item", id: "fullscreen", label: "全屏", icon: "fullscreen", shortcut: shortcutBindings.toggleFullscreen, onSelect: toggleFullscreen },
    { type: "item", id: "settings", label: "设置", icon: "settings", shortcut: shortcutBindings.openSettings, onSelect: openSettingsDialog },
    { type: "separator", id: "window-separator" },
    { type: "item", id: "close", label: "关闭窗口", icon: "close", onSelect: () => runWindowCommand("window_close") },
  ];

  function renderTrackList(kind: SelectableTrackKind, label: string, items: MpvTrack[]) {
    const hasSelected = items.some((track) => track.selected);

    return (
      <section className="media-panel-section">
        <header>
          <h3>{label}</h3>
          <span>{items.length ? `${items.length} 条` : "无"}</span>
        </header>
        <div className="track-list">
          {kind === "subtitle" && (
            <button className={`track-item ${hasSelected ? "" : "track-item--active"}`} type="button" onClick={() => selectTrack(kind, null)}>
              <span>关闭字幕</span>
              <small>Off</small>
            </button>
          )}
          {items.map((track) => (
            <button
              key={`${track.kind}:${track.id}`}
              className={`track-item ${track.selected ? "track-item--active" : ""}`}
              type="button"
              onClick={() => selectTrack(kind, track.id)}
            >
              <span>{trackDisplayLabel(track)}</span>
              <small>ID {track.id}</small>
            </button>
          ))}
          {!items.length && kind !== "subtitle" && <div className="track-empty">当前媒体未报告可切换轨道</div>}
        </div>
      </section>
    );
  }

  return (
    <main className="app-shell" onContextMenu={openContextMenu} onKeyDown={recordUserActivity} onPointerDown={handleShellPointerDown} onPointerLeave={handleShellPointerLeave} onPointerMove={recordUserActivity}>
      <section className={`window-shell ${media ? "window-shell--loaded" : ""}`} aria-label="OpenPlayer">
        <section className={`stage ${media ? "stage--loaded" : ""} ${isChromeHidden ? "stage--chrome-hidden" : ""}`} aria-label="Player surface">
          {!media && (
            <div className="empty-open">
              <img className="empty-open-logo" src={openPlayerLogoUrl} alt="" draggable={false} />
              <span>Open media</span>
            </div>
          )}

          <div className="drag-region" data-tauri-drag-region aria-hidden="true" onAuxClick={(event) => event.preventDefault()} onPointerDown={handleDragRegionPointerDown} />

          {resizeRegions.map((region) => (
            <div
              key={region.direction}
              aria-hidden="true"
              className={`resize-region ${region.className}`}
              onPointerDown={(event) => handleResizePointerDown(event, region.direction)}
            />
          ))}

          <div className="window-controls" aria-label="Window controls">
            <button type="button" aria-label="Minimize window" onClick={() => runWindowCommand("window_minimize")}>
              <Icon name="minimize" />
            </button>
            <button type="button" aria-label="Maximize or restore window" onClick={() => runWindowCommand("window_toggle_maximize")}>
              <Icon name="maximize" />
            </button>
            <button className="window-control-close" type="button" aria-label="Close window" onClick={() => runWindowCommand("window_close")}>
              <Icon name="close" />
            </button>
          </div>

          {playbackError && <div className="playback-error" role="alert">{playbackError}</div>}

          <div className="transport" aria-label="Playback controls">
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
                  aria-label="Seek playback position"
                  onChange={(event) => seekTo(Number(event.currentTarget.value))}
                  onPointerUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                  onKeyUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                  onBlur={(event) => commitSeekTo(Number(event.currentTarget.value))}
                  disabled={!media || duration <= 0}
                />
              </div>
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
              <button type="button" aria-label="Open media" onClick={openNativeMediaFiles} disabled={isPickerOpen}>
                <Icon name="folder" />
              </button>
              <button className="control-primary" type="button" aria-label={isPlaying ? "Pause" : media ? "Play" : "Open media"} onClick={togglePlayback} disabled={!media && isPickerOpen}>
                <Icon name={isPlaying ? "pause" : "play"} />
              </button>
              <button type="button" aria-label="Restart" onClick={() => commitSeekTo(0)} disabled={!media}>
                <Icon name="restart" />
              </button>
              <label className="volume-control" aria-label="Volume">
                <Icon name="volume" />
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={volumeLevel}
                  aria-label="Volume"
                  onChange={(event) => setVolume(Number(event.currentTarget.value))}
                />
              </label>
              <button className="speed-toggle" type="button" aria-label="Open playback speed and track options" aria-expanded={isMediaPanelOpen} onClick={toggleMediaPanel} disabled={!media}>
                {formatPlaybackSpeed(playbackSpeed)}
              </button>
              <button
                className={`playlist-toggle ${isPlaylistOpen ? "playlist-toggle--open" : ""}`}
                type="button"
                aria-label="Toggle playlist"
                aria-expanded={isPlaylistOpen}
                onClick={togglePlaylist}
              >
                <Icon name="list" />
              </button>
              <button type="button" aria-label="Open settings" onClick={openSettingsDialog}>
                <Icon name="settings" />
              </button>
            </div>
          </div>

          {isMediaPanelOpen && media && (
            <aside
              className="media-panel"
              aria-label="Playback speed and track options"
              onContextMenu={(event) => event.stopPropagation()}
              onPointerDown={(event) => event.stopPropagation()}
            >
              <section className="media-panel-section">
                <header>
                  <h3>播放速度</h3>
                  <span>{formatPlaybackSpeed(playbackSpeed)}</span>
                </header>
                <div className="speed-options" role="group" aria-label="Playback speed">
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

              {renderTrackList("audio", "音轨", audioTracks)}
              {renderTrackList("video", "视频轨", videoTracks)}
              {renderTrackList("subtitle", "字幕", subtitleTracks)}

              <button className="subtitle-load" type="button" onClick={addExternalSubtitle} disabled={isPickerOpen}>
                加载外部字幕
              </button>
            </aside>
          )}

          {isPlaylistOpen && (
            <aside className="playlist-drawer playlist-drawer--open" aria-label="Playlist">
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
            </aside>
          )}

          {contextMenu && (
            <div
              className="context-menu"
              role="menu"
              aria-label="Player context menu"
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
                    {item.shortcut && <kbd>{formatShortcutChord(item.shortcut)}</kbd>}
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
                    <h2 id="settings-title">设置</h2>
                  </div>
                  <button className="settings-close" type="button" aria-label="Close settings" onClick={closeSettingsDialog}>
                    <Icon name="close" />
                  </button>
                </header>

                <div className="settings-layout">
                  <nav className="settings-nav" aria-label="Settings sections">
                    <button className="settings-nav-item settings-nav-item--active" type="button" aria-current="page">
                      <Icon name="settings" />
                      <span>快捷键</span>
                    </button>
                  </nav>

                  <section className="settings-panel" aria-labelledby="shortcut-settings-title">
                    <div className="settings-panel-heading">
                      <div>
                        <h3 id="shortcut-settings-title">快捷键</h3>
                        <span>{recordingShortcutAction ? "输入中" : "自定义控制"}</span>
                      </div>
                      <button className="settings-reset" type="button" onClick={resetShortcutBindings}>
                        恢复默认
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
                                <kbd>{isRecording ? "按键中" : formatShortcutChord(binding)}</kbd>
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
