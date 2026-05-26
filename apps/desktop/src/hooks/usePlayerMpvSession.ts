import { useRef, type MutableRefObject } from "react";
import { applyMpvSnapshotToPlaybackState, openMpvPathForPlaybackSession } from "../app/mpvSession";
import {
  commitMpvSeek,
  previewMpvSeek,
  restartMpvPlaybackSession,
  seekTargetForDuration,
  stopMpvPlaybackSession,
  toggleMpvPlaybackSession,
} from "../app/mpvSessionCommands";
import type {
  HardwareDecodingMode,
  MediaItem,
  MediaPanelMode,
  MpvLoadOptions,
  MpvRecordingState,
  MpvSnapshot,
  MpvTrack,
  PendingSeek,
  PlaybackHistoryEntry,
  PlaybackSettings,
  PlayerPreferences,
  PluginMediaOpenInput,
  PluginMediaOpenResult,
} from "../app/types";
import { useMpvSnapshotPolling } from "./useMpvSnapshotPolling";
import { usePlaybackClock } from "./usePlaybackClock";
import { usePlaybackHistoryProgress } from "./usePlaybackHistoryProgress";

type PreparedMediaOpen = {
  item: MediaItem;
  loadOptions: MpvLoadOptions;
};

type PlayerMpvSessionHandlers = {
  applyStoredPluginMpvSettings: (snapshot: MpvSnapshot) => Promise<MpvSnapshot>;
  broadcastPluginRuntimeEvent: (event: string, payload: unknown) => void;
  handlePlaybackEnd: (path: string) => void;
  openNativeMediaFiles: () => void;
  runMediaOpeningHooks: (input: PluginMediaOpenInput) => Promise<PluginMediaOpenResult>;
};

type UsePlayerMpvSessionOptions = {
  media: MediaItem | null;
  playerPreferences: PlayerPreferences;
  duration: number;
  isPlaying: boolean;
  playbackSpeed: number;
  pendingSeekRef: MutableRefObject<PendingSeek | null>;
  handledEndedPathRef: MutableRefObject<string | null>;
  hardwareDecodingModeRef: MutableRefObject<HardwareDecodingMode>;
  previousAudibleVolumeRef: MutableRefObject<number>;
  setDuration: (value: number) => void;
  setIsPlaying: (value: boolean) => void;
  setFramesPerSecond: (value: number) => void;
  setPlaybackSpeedValue: (value: number) => void;
  setHardwareDecodingModeValue: (value: HardwareDecodingMode) => void;
  setIsVideoFillEnabled: (value: boolean) => void;
  setSubtitleDelayValue: (value: number) => void;
  setTracks: (value: MpvTrack[]) => void;
  setLoadedMediaPath: (value: string | null) => void;
  setVolumeLevel: (value: number) => void;
  setCurrentTime: (value: number) => void;
  setCurrentIndex: (value: number | null) => void;
  setMediaPanelMode: (value: MediaPanelMode | null) => void;
  setPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void;
  setRecordingState: (state: MpvRecordingState) => void;
  setPlaybackError: (message: string | null) => void;
  loadPlaybackSettings: () => Promise<PlaybackSettings>;
  onError: (error: unknown) => void;
};

const passthroughMediaOpen = async (input: PluginMediaOpenInput): Promise<PluginMediaOpenResult> => input;

export function usePlayerMpvSession({
  media,
  playerPreferences,
  duration,
  isPlaying,
  playbackSpeed,
  pendingSeekRef,
  handledEndedPathRef,
  hardwareDecodingModeRef,
  previousAudibleVolumeRef,
  setDuration,
  setIsPlaying,
  setFramesPerSecond,
  setPlaybackSpeedValue,
  setHardwareDecodingModeValue,
  setIsVideoFillEnabled,
  setSubtitleDelayValue,
  setTracks,
  setLoadedMediaPath,
  setVolumeLevel,
  setCurrentTime,
  setCurrentIndex,
  setMediaPanelMode,
  setPlaybackHistory,
  setRecordingState,
  setPlaybackError,
  loadPlaybackSettings,
  onError,
}: UsePlayerMpvSessionOptions) {
  const handlersRef = useRef<PlayerMpvSessionHandlers>({
    applyStoredPluginMpvSettings: async (snapshot) => snapshot,
    broadcastPluginRuntimeEvent: () => undefined,
    handlePlaybackEnd: () => undefined,
    openNativeMediaFiles: () => undefined,
    runMediaOpeningHooks: passthroughMediaOpen,
  });
  const { invalidatePendingSnapshots } = useMpvSnapshotPolling({
    mediaId: media?.id,
    applySnapshot,
  });
  const { displayPosition, anchorDisplayClock } = usePlaybackClock({
    mediaId: media?.id,
    isPlaying,
    duration,
    playbackSpeed,
  });
  const { rememberPlaybackProgress } = usePlaybackHistoryProgress({
    playerPreferences,
    setPlaybackHistory,
  });

  function bindPlayerMpvSessionHandlers(handlers: Partial<PlayerMpvSessionHandlers>) {
    handlersRef.current = {
      ...handlersRef.current,
      ...handlers,
    };
  }

  function applyCommandSnapshot(snapshot: MpvSnapshot) {
    invalidatePendingSnapshots();
    applySnapshot(snapshot, true);
  }

  function applySnapshot(snapshot: MpvSnapshot, forceHistoryWrite = false) {
    applyMpvSnapshotToPlaybackState({
      snapshot,
      forceHistoryWrite,
      pendingSeekRef,
      handledEndedPathRef,
      hardwareDecodingModeRef,
      previousAudibleVolumeRef,
      setDuration,
      setIsPlaying,
      setFramesPerSecond,
      setPlaybackSpeedValue,
      setHardwareDecodingModeValue,
      setIsVideoFillEnabled,
      setSubtitleDelayValue,
      setTracks,
      setLoadedMediaPath,
      setVolumeLevel,
      setCurrentTime,
      rememberPlaybackProgress,
      anchorDisplayClock,
      broadcastPluginRuntimeEvent: handlersRef.current.broadcastPluginRuntimeEvent,
      handlePlaybackEnd: handlersRef.current.handlePlaybackEnd,
    });
  }

  async function preparePluginMediaOpen(item: MediaItem, source: PluginMediaOpenInput["source"], loadOptions: MpvLoadOptions = {}): Promise<PreparedMediaOpen> {
    const prepared = await handlersRef.current.runMediaOpeningHooks({
      path: item.path,
      name: item.name,
      source,
      loadOptions,
    });
    return {
      item: {
        ...item,
        path: prepared.path,
        name: prepared.name,
      },
      loadOptions: prepared.loadOptions,
    };
  }

  async function openMpvPath(path: string, loadOptions: MpvLoadOptions = {}) {
    await openMpvPathForPlaybackSession({
      path,
      loadOptions,
      pendingSeekRef,
      handledEndedPathRef,
      invalidatePendingSnapshots,
      setLoadedMediaPath,
      loadPlaybackSettings,
      setRecordingState,
      applyStoredPluginMpvSettings: handlersRef.current.applyStoredPluginMpvSettings,
      setPlaybackError,
      broadcastPluginRuntimeEvent: handlersRef.current.broadcastPluginRuntimeEvent,
      applyCommandSnapshot,
    });
  }

  function seekTarget(value: number) {
    return seekTargetForDuration(value, duration);
  }

  function stopPlayback() {
    stopMpvPlaybackSession({
      media,
      pendingSeekRef,
      handledEndedPathRef,
      playbackSpeed,
      invalidatePendingSnapshots,
      setCurrentIndex,
      setIsPlaying,
      setDuration,
      setCurrentTime,
      anchorDisplayClock,
      setFramesPerSecond,
      setTracks,
      setLoadedMediaPath,
      setMediaPanelMode,
      broadcastPluginRuntimeEvent: handlersRef.current.broadcastPluginRuntimeEvent,
      onError,
    });
  }

  function restartPlayback(autoplay = false) {
    restartMpvPlaybackSession({
      media,
      autoplay,
      duration,
      playbackSpeed,
      pendingSeekRef,
      handledEndedPathRef,
      setCurrentTime,
      anchorDisplayClock,
      invalidatePendingSnapshots,
      applyCommandSnapshot,
      onError,
    });
  }

  function togglePlayback() {
    toggleMpvPlaybackSession({
      media,
      isPlaying,
      openNativeMediaFiles: handlersRef.current.openNativeMediaFiles,
      invalidatePendingSnapshots,
      applyCommandSnapshot,
      onError,
    });
  }

  function seekTo(value: number) {
    previewMpvSeek({
      value,
      duration,
      pendingSeekRef,
      setCurrentTime,
      anchorDisplayClock,
    });
  }

  function commitSeekTo(value: number) {
    commitMpvSeek({
      value,
      duration,
      pendingSeekRef,
      setCurrentTime,
      anchorDisplayClock,
      invalidatePendingSnapshots,
      applyCommandSnapshot,
      onError,
    });
  }

  function clearPendingSeek() {
    pendingSeekRef.current = null;
  }

  return {
    displayPosition,
    anchorDisplayClock,
    invalidatePendingSnapshots,
    applyCommandSnapshot,
    applySnapshot,
    preparePluginMediaOpen,
    openMpvPath,
    seekTarget,
    stopPlayback,
    restartPlayback,
    togglePlayback,
    seekTo,
    commitSeekTo,
    clearPendingSeek,
    bindPlayerMpvSessionHandlers,
  };
}
