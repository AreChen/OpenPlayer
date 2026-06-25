import { useRef, useState } from "react";
import {
  DEFAULT_PLAYBACK_SETTINGS,
  DEFAULT_PLAYER_PREFERENCES,
  INACTIVE_RECORDING_STATE,
} from "../app/constants";
import type {
  AppVersionInfo,
  AppearanceState,
  HardwareDecodingMode,
  LoopMode,
  MediaItem,
  MpvRecordingState,
  MpvTrack,
  PendingSeek,
  PlatformSupport,
  PlaybackHistoryEntry,
  PlayerPreferences,
  PluginRuntimeLogEntry,
  TimeDisplayMode,
} from "../app/types";

export function usePlayerOverlayState() {
  const [queue, setQueue] = useState<MediaItem[]>([]);
  const [currentIndex, setCurrentIndex] = useState<number | null>(null);
  const [playbackHistory, setPlaybackHistory] = useState<PlaybackHistoryEntry[]>([]);
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [volumeLevel, setVolumeLevel] = useState(0.82);
  const [playbackSpeed, setPlaybackSpeedValue] = useState(1);
  const [hardwareDecodingMode, setHardwareDecodingModeValue] =
    useState<HardwareDecodingMode>("hardware");
  const [isVideoFillEnabled, setIsVideoFillEnabled] = useState(false);
  const [subtitleDelay, setSubtitleDelayValue] = useState(0);
  const [tracks, setTracks] = useState<MpvTrack[]>([]);
  const [loadedMediaPath, setLoadedMediaPath] = useState<string | null>(null);
  const [framesPerSecond, setFramesPerSecond] = useState(0);
  const [timeDisplayMode, setTimeDisplayModeValue] = useState<TimeDisplayMode>("timecode");
  const [loopMode, setLoopModeValue] = useState<LoopMode>("off");
  const [isPlaying, setIsPlaying] = useState(false);
  const [isAlwaysOnTop, setIsAlwaysOnTop] = useState(false);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [platformSupport, setPlatformSupport] = useState<PlatformSupport | null>(null);
  const [appearanceState, setAppearanceState] = useState<AppearanceState | null>(null);
  const [playerPreferences, setPlayerPreferences] =
    useState<PlayerPreferences>(DEFAULT_PLAYER_PREFERENCES);
  const [appVersion, setAppVersion] = useState<AppVersionInfo | null>(null);
  const [recordingState, setRecordingState] =
    useState<MpvRecordingState>(INACTIVE_RECORDING_STATE);
  const [systemFontFamilies, setSystemFontFamilies] = useState<string[]>([]);
  const [pluginRuntimeLogs, setPluginRuntimeLogs] = useState<PluginRuntimeLogEntry[]>([]);
  const pendingSeekRef = useRef<PendingSeek | null>(null);
  const handledEndedPathRef = useRef<string | null>(null);
  const hardwareDecodingModeRef = useRef<HardwareDecodingMode>("hardware");
  const previousAudibleVolumeRef = useRef(DEFAULT_PLAYBACK_SETTINGS.volume / 100);
  const clearResizeHoverCursorRef = useRef<() => void>(() => undefined);

  return {
    queue,
    setQueue,
    currentIndex,
    setCurrentIndex,
    playbackHistory,
    setPlaybackHistory,
    duration,
    setDuration,
    currentTime,
    setCurrentTime,
    volumeLevel,
    setVolumeLevel,
    playbackSpeed,
    setPlaybackSpeedValue,
    hardwareDecodingMode,
    setHardwareDecodingModeValue,
    isVideoFillEnabled,
    setIsVideoFillEnabled,
    subtitleDelay,
    setSubtitleDelayValue,
    tracks,
    setTracks,
    loadedMediaPath,
    setLoadedMediaPath,
    framesPerSecond,
    setFramesPerSecond,
    timeDisplayMode,
    setTimeDisplayModeValue,
    loopMode,
    setLoopModeValue,
    isPlaying,
    setIsPlaying,
    isAlwaysOnTop,
    setIsAlwaysOnTop,
    isPickerOpen,
    setIsPickerOpen,
    platformSupport,
    setPlatformSupport,
    appearanceState,
    setAppearanceState,
    playerPreferences,
    setPlayerPreferences,
    appVersion,
    setAppVersion,
    recordingState,
    setRecordingState,
    systemFontFamilies,
    setSystemFontFamilies,
    pluginRuntimeLogs,
    setPluginRuntimeLogs,
    pendingSeekRef,
    handledEndedPathRef,
    hardwareDecodingModeRef,
    previousAudibleVolumeRef,
    clearResizeHoverCursorRef,
  };
}
