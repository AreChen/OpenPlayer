import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_SEEK_STEP_SECONDS, DEFAULT_VOLUME_STEP } from "../app/constants";
import type { MediaItem, MpvSnapshot, ShortcutAction } from "../app/types";

type UsePlaybackShortcutActionsOptions = {
  media: MediaItem | null;
  queueLength: number;
  duration: number;
  displayTime: number;
  volumeLevel: number;
  openNativeMediaFiles: () => void;
  togglePlayback: () => void;
  restartPlayback: () => void;
  togglePlaylist: () => void;
  setVolume: (value: number, options?: { feedback?: boolean }) => void;
  toggleFullscreen: () => void;
  toggleAlwaysOnTop: () => void;
  openSettingsDialog: () => void;
  commitSeekTo: (value: number) => void;
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  clearPendingSeek: () => void;
  onError: (error: unknown) => void;
};

export function usePlaybackShortcutActions({
  media,
  queueLength,
  duration,
  displayTime,
  volumeLevel,
  openNativeMediaFiles,
  togglePlayback,
  restartPlayback,
  togglePlaylist,
  setVolume,
  toggleFullscreen,
  toggleAlwaysOnTop,
  openSettingsDialog,
  commitSeekTo,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  clearPendingSeek,
  onError,
}: UsePlaybackShortcutActionsOptions) {
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

    clearPendingSeek();
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>(command).then(applyCommandSnapshot).catch(onError);
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
        if (media || queueLength > 0) {
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
      case "toggleAlwaysOnTop":
        toggleAlwaysOnTop();
        break;
      case "openSettings":
        openSettingsDialog();
        break;
    }
  }

  return {
    performShortcutAction,
  };
}
