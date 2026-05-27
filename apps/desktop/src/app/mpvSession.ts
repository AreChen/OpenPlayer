import { invoke } from "@tauri-apps/api/core";
import {
  INACTIVE_RECORDING_STATE,
  SEEK_CONFIRM_TOLERANCE_SECONDS,
  SEEK_SNAPSHOT_SUPPRESS_MS,
} from "./constants";
import { applyStoredMediaPlaybackSettingsToMpv, applyStoredPlaybackSettingsToMpv } from "./mpvLoadSettings";
import {
  clampPlaybackSpeed,
  clampSubtitleDelay,
  hwdecModeFromSnapshot,
  resumePositionForPath,
} from "./playback";
import type {
  HardwareDecodingMode,
  MpvLoadOptions,
  MpvRecordingState,
  MpvSnapshot,
  MpvTrack,
  PendingSeek,
  PlaybackSettings,
} from "./types";

type RefValue<T> = {
  current: T;
};

type SetValue<T> = (value: T) => void;

type PlaybackEventSnapshot = {
  path: string | null;
  playing: boolean;
  volume: number;
  speed: number;
  tracks: string;
};

type ApplyMpvSnapshotOptions = {
  snapshot: MpvSnapshot;
  forceHistoryWrite?: boolean;
  pendingSeekRef: RefValue<PendingSeek | null>;
  handledEndedPathRef: RefValue<string | null>;
  hardwareDecodingModeRef: RefValue<HardwareDecodingMode>;
  previousAudibleVolumeRef: RefValue<number>;
  previousPlaybackEventRef: RefValue<PlaybackEventSnapshot | null>;
  setDuration: SetValue<number>;
  setIsPlaying: SetValue<boolean>;
  setFramesPerSecond: SetValue<number>;
  setPlaybackSpeedValue: SetValue<number>;
  setHardwareDecodingModeValue: SetValue<HardwareDecodingMode>;
  setIsVideoFillEnabled: SetValue<boolean>;
  setSubtitleDelayValue: SetValue<number>;
  setTracks: SetValue<MpvTrack[]>;
  setLoadedMediaPath: SetValue<string | null>;
  setVolumeLevel: SetValue<number>;
  setCurrentTime: SetValue<number>;
  rememberPlaybackProgress: (path: string, position: number, duration: number, forceWrite?: boolean) => void;
  anchorDisplayClock: (position: number, playing: boolean, upperDuration?: number, speed?: number) => void;
  broadcastPluginRuntimeEvent: (event: string, payload: unknown) => void;
  handlePlaybackEnd: (path: string) => void;
};

export function applyMpvSnapshotToPlaybackState({
  snapshot,
  forceHistoryWrite = false,
  pendingSeekRef,
  handledEndedPathRef,
  hardwareDecodingModeRef,
  previousAudibleVolumeRef,
  previousPlaybackEventRef,
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
  broadcastPluginRuntimeEvent,
  handlePlaybackEnd,
}: ApplyMpvSnapshotOptions) {
  const snapshotPosition = Number.isFinite(snapshot.position) ? snapshot.position : 0;
  const snapshotDuration = Number.isFinite(snapshot.duration) ? snapshot.duration : 0;
  const snapshotSpeed = clampPlaybackSpeed(snapshot.speed);
  const pendingSeek = pendingSeekRef.current;
  let confirmedSeekTarget: number | null = null;
  const nextIsPlaying = !snapshot.paused && snapshot.status === "playing";
  const snapshotHwdecMode = hwdecModeFromSnapshot(snapshot.hwdec);

  setDuration(snapshotDuration);
  setIsPlaying(nextIsPlaying);
  setFramesPerSecond(Number.isFinite(snapshot.fps) && snapshot.fps > 0 ? snapshot.fps : 0);
  setPlaybackSpeedValue(snapshotSpeed);
  setHardwareDecodingModeValue(snapshotHwdecMode);
  hardwareDecodingModeRef.current = snapshotHwdecMode;
  setIsVideoFillEnabled(snapshot.videoFill === true);
  setSubtitleDelayValue(clampSubtitleDelay(snapshot.subtitleDelay));
  setTracks(Array.isArray(snapshot.tracks) ? snapshot.tracks : []);
  setLoadedMediaPath(snapshot.path);

  const snapshotVolume = Math.min(1, Math.max(0, snapshot.volume / 100));
  setVolumeLevel(snapshotVolume);
  if (snapshotVolume > 0) {
    previousAudibleVolumeRef.current = snapshotVolume;
  }

  if (pendingSeek) {
    const isConfirmed = Math.abs(snapshotPosition - pendingSeek.target) <= SEEK_CONFIRM_TOLERANCE_SECONDS;
    const isExpired = performance.now() - pendingSeek.startedAt > SEEK_SNAPSHOT_SUPPRESS_MS;
    if (!isConfirmed && !isExpired) {
      return;
    }

    if (isConfirmed) {
      confirmedSeekTarget = pendingSeek.target;
    }
    pendingSeekRef.current = null;
  }

  rememberPlaybackProgress(snapshot.path, snapshotPosition, snapshotDuration, forceHistoryWrite);
  setCurrentTime(snapshotPosition);
  anchorDisplayClock(snapshotPosition, nextIsPlaying, snapshotDuration, snapshotSpeed);
  broadcastPluginRuntimeEvent("playback.snapshot", {
    ...snapshot,
    position: snapshotPosition,
    duration: snapshotDuration,
    playing: nextIsPlaying,
  });
  broadcastPlaybackTransitionEvents({
    snapshot,
    path: snapshot.path,
    playing: nextIsPlaying,
    position: snapshotPosition,
    duration: snapshotDuration,
    volume: snapshot.volume,
    speed: snapshotSpeed,
    tracks: Array.isArray(snapshot.tracks) ? snapshot.tracks : [],
    confirmedSeekTarget,
    previousPlaybackEventRef,
    broadcastPluginRuntimeEvent,
  });

  if (snapshot.ended || snapshot.status === "ended") {
    broadcastPluginRuntimeEvent("playback.ended", { path: snapshot.path });
    handlePlaybackEnd(snapshot.path);
  } else if (handledEndedPathRef.current === snapshot.path) {
    handledEndedPathRef.current = null;
  }
}

function broadcastPlaybackTransitionEvents({
  snapshot,
  path,
  playing,
  position,
  duration,
  volume,
  speed,
  tracks,
  confirmedSeekTarget,
  previousPlaybackEventRef,
  broadcastPluginRuntimeEvent,
}: {
  snapshot: MpvSnapshot;
  path: string | null;
  playing: boolean;
  position: number;
  duration: number;
  volume: number;
  speed: number;
  tracks: MpvTrack[];
  confirmedSeekTarget: number | null;
  previousPlaybackEventRef: RefValue<PlaybackEventSnapshot | null>;
  broadcastPluginRuntimeEvent: (event: string, payload: unknown) => void;
}) {
  const trackSignature = JSON.stringify(
    tracks.map((track) => ({
      id: track.id,
      kind: track.kind,
      selected: track.selected,
      title: track.title,
      language: track.language,
    })),
  );
  const previous = previousPlaybackEventRef.current;
  if (!previous || previous.path !== path) {
    if (playing) {
      broadcastPluginRuntimeEvent("playback.started", { path, position, duration, snapshot });
    }
  } else {
    if (!previous.playing && playing) {
      broadcastPluginRuntimeEvent("playback.started", { path, position, duration, snapshot });
    }
    if (previous.playing && !playing && !snapshot.ended && snapshot.status !== "ended") {
      broadcastPluginRuntimeEvent("playback.paused", { path, position, duration, snapshot });
    }
    if (previous.volume !== volume) {
      broadcastPluginRuntimeEvent("playback.volumeChanged", { path, volume });
    }
    if (previous.speed !== speed) {
      broadcastPluginRuntimeEvent("playback.speedChanged", { path, speed });
    }
    if (previous.tracks !== trackSignature) {
      broadcastPluginRuntimeEvent("tracks.changed", { path, tracks });
    }
  }
  if (confirmedSeekTarget !== null) {
    broadcastPluginRuntimeEvent("playback.seeked", { path, position, target: confirmedSeekTarget });
  }
  previousPlaybackEventRef.current = {
    path,
    playing,
    volume,
    speed,
    tracks: trackSignature,
  };
}

type OpenMpvPathOptions = {
  path: string;
  loadOptions?: MpvLoadOptions;
  pendingSeekRef: RefValue<PendingSeek | null>;
  handledEndedPathRef: RefValue<string | null>;
  invalidatePendingSnapshots: () => void;
  setLoadedMediaPath: SetValue<string | null>;
  loadPlaybackSettings: () => Promise<PlaybackSettings>;
  setRecordingState: SetValue<MpvRecordingState>;
  applyStoredPluginMpvSettings: (snapshot: MpvSnapshot) => Promise<MpvSnapshot>;
  setPlaybackError: SetValue<string | null>;
  broadcastPluginRuntimeEvent: (event: string, payload: unknown) => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
};

export async function openMpvPathForPlaybackSession({
  path,
  loadOptions = {},
  pendingSeekRef,
  handledEndedPathRef,
  invalidatePendingSnapshots,
  setLoadedMediaPath,
  loadPlaybackSettings,
  setRecordingState,
  applyStoredPluginMpvSettings,
  setPlaybackError,
  broadcastPluginRuntimeEvent,
  applyCommandSnapshot,
}: OpenMpvPathOptions) {
  invalidatePendingSnapshots();
  handledEndedPathRef.current = null;
  setLoadedMediaPath(null);

  const savedSettings = await loadPlaybackSettings();
  const rememberedPosition = await resumePositionForPath(path);
  let activeSnapshot = await invoke<MpvSnapshot>("mpv_overlay_open_path", {
    path,
    resumePosition: rememberedPosition > 0 ? rememberedPosition : null,
    initialVolume: savedSettings.volume,
    loadOptions,
  });

  setRecordingState(INACTIVE_RECORDING_STATE);
  activeSnapshot = await applyStoredPlaybackSettingsToMpv(activeSnapshot, savedSettings);
  activeSnapshot = await applyStoredMediaPlaybackSettingsToMpv(path, activeSnapshot);
  activeSnapshot = await applyStoredPluginMpvSettings(activeSnapshot);
  pendingSeekRef.current = null;
  setPlaybackError(null);
  broadcastPluginRuntimeEvent("media.loaded", { path: activeSnapshot.path, snapshot: activeSnapshot });
  applyCommandSnapshot(activeSnapshot);
}
