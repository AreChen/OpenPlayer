import { invoke } from "@tauri-apps/api/core";
import type {
  MediaItem,
  MediaPanelMode,
  MpvSnapshot,
  MpvTrack,
  PendingSeek,
} from "../types";
import type {
  AnchorDisplayClock,
  RefValue,
  ReportError,
  SetValue,
} from "./shared";

type StopPlaybackOptions = {
  media: MediaItem | null;
  pendingSeekRef: RefValue<PendingSeek | null>;
  handledEndedPathRef: RefValue<string | null>;
  playbackSpeed: number;
  invalidatePendingSnapshots: () => void;
  setCurrentIndex: SetValue<number | null>;
  setIsPlaying: SetValue<boolean>;
  setDuration: SetValue<number>;
  setCurrentTime: SetValue<number>;
  anchorDisplayClock: AnchorDisplayClock;
  setFramesPerSecond: SetValue<number>;
  setTracks: SetValue<MpvTrack[]>;
  setLoadedMediaPath: SetValue<string | null>;
  setMediaPanelMode: SetValue<MediaPanelMode | null>;
  broadcastPluginRuntimeEvent: (event: string, payload: unknown) => void;
  onError: ReportError;
};

export function stopMpvPlaybackSession({
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
  broadcastPluginRuntimeEvent,
  onError,
}: StopPlaybackOptions) {
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
      anchorDisplayClock(0, false, 0, playbackSpeed);
      setFramesPerSecond(0);
      setTracks([]);
      setLoadedMediaPath(null);
      setMediaPanelMode(null);
      broadcastPluginRuntimeEvent("playback.stopped", { path: media.path });
    })
    .catch(onError);
}

type RestartPlaybackOptions = {
  media: MediaItem | null;
  autoplay?: boolean;
  duration: number;
  playbackSpeed: number;
  pendingSeekRef: RefValue<PendingSeek | null>;
  handledEndedPathRef: RefValue<string | null>;
  setCurrentTime: SetValue<number>;
  anchorDisplayClock: AnchorDisplayClock;
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  onError: ReportError;
};

export function restartMpvPlaybackSession({
  media,
  autoplay = false,
  duration,
  playbackSpeed,
  pendingSeekRef,
  handledEndedPathRef,
  setCurrentTime,
  anchorDisplayClock,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  onError,
}: RestartPlaybackOptions) {
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
      onError(error);
    });
}

type TogglePlaybackOptions = {
  media: MediaItem | null;
  isPlaying: boolean;
  openNativeMediaFiles: () => void | Promise<void>;
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  onError: ReportError;
};

export function toggleMpvPlaybackSession({
  media,
  isPlaying,
  openNativeMediaFiles,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  onError,
}: TogglePlaybackOptions) {
  if (!media) {
    openNativeMediaFiles();
    return;
  }

  invalidatePendingSnapshots();
  invoke<MpvSnapshot>(isPlaying ? "mpv_embed_pause" : "mpv_embed_play")
    .then(applyCommandSnapshot)
    .catch(onError);
}
