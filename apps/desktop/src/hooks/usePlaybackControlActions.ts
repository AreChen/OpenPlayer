import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_PLAYBACK_SETTINGS, DEFAULT_VOLUME_STEP } from "../app/constants";
import { canDisplayFrames, clampPlaybackSpeed } from "../app/playback";
import type {
  HardwareDecodingMode,
  LoopMode,
  MediaItem,
  MpvSnapshot,
  PlaybackSettingsUpdate,
  TimeDisplayMode,
} from "../app/types";
import type { MutableRefObject } from "react";

type VolumeOptions = {
  feedback?: boolean;
};

type UsePlaybackControlActionsOptions = {
  media: MediaItem | null;
  duration: number;
  displayTime: number;
  framesPerSecond: number;
  isPlaying: boolean;
  volumeLevel: number;
  hardwareDecodingMode: HardwareDecodingMode;
  isVideoFillEnabled: boolean;
  timeDisplayMode: TimeDisplayMode;
  previousAudibleVolumeRef: MutableRefObject<number>;
  hardwareDecodingModeRef: MutableRefObject<HardwareDecodingMode>;
  setVolumeLevel: (volume: number) => void;
  setPlaybackSpeedValue: (speed: number) => void;
  setHardwareDecodingModeValue: (mode: HardwareDecodingMode) => void;
  setIsVideoFillEnabled: (enabled: boolean) => void;
  setTimeDisplayModeValue: (mode: TimeDisplayMode) => void;
  setLoopModeValue: (mode: LoopMode) => void;
  persistPlaybackSettings: (update: PlaybackSettingsUpdate) => void;
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  anchorDisplayClock: (position: number, playing: boolean, upperDuration?: number, speed?: number) => void;
  showVolumeFeedback: (level: number) => void;
  onError: (error: unknown) => void;
};

export function usePlaybackControlActions({
  media,
  duration,
  displayTime,
  framesPerSecond,
  isPlaying,
  volumeLevel,
  hardwareDecodingMode,
  isVideoFillEnabled,
  timeDisplayMode,
  previousAudibleVolumeRef,
  hardwareDecodingModeRef,
  setVolumeLevel,
  setPlaybackSpeedValue,
  setHardwareDecodingModeValue,
  setIsVideoFillEnabled,
  setTimeDisplayModeValue,
  setLoopModeValue,
  persistPlaybackSettings,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  anchorDisplayClock,
  showVolumeFeedback,
  onError,
}: UsePlaybackControlActionsOptions) {
  function toggleTimeDisplayMode() {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayModeValue("timecode");
      persistPlaybackSettings({ timeDisplayMode: "timecode" });
      return;
    }

    const nextMode = timeDisplayMode === "timecode" ? "frames" : "timecode";
    setTimeDisplayModeValue(nextMode);
    persistPlaybackSettings({ timeDisplayMode: nextMode });
  }

  function setVolume(value: number, options: VolumeOptions = {}) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
    if (nextVolume > 0) {
      previousAudibleVolumeRef.current = nextVolume;
    }
    persistPlaybackSettings({ volume: nextVolume * 100 });
    if (options.feedback) {
      showVolumeFeedback(nextVolume);
    }
    if (!media) {
      return;
    }
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_volume", { volume: nextVolume * 100 })
      .then(applyCommandSnapshot)
      .catch(onError);
  }

  function toggleMute() {
    if (volumeLevel > 0) {
      previousAudibleVolumeRef.current = volumeLevel;
      setVolume(0, { feedback: true });
      return;
    }

    const restoredVolume = Math.min(
      1,
      Math.max(DEFAULT_VOLUME_STEP, previousAudibleVolumeRef.current || DEFAULT_PLAYBACK_SETTINGS.volume / 100),
    );
    setVolume(restoredVolume, { feedback: true });
  }

  function setPlaybackSpeed(speed: number) {
    const nextSpeed = clampPlaybackSpeed(speed);
    setPlaybackSpeedValue(nextSpeed);
    persistPlaybackSettings({ playbackSpeed: nextSpeed });
    if (!media) {
      return;
    }

    anchorDisplayClock(displayTime, isPlaying, duration, nextSpeed);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_speed", { speed: nextSpeed })
      .then(applyCommandSnapshot)
      .catch(onError);
  }

  function toggleHardwareDecoding() {
    const nextMode: HardwareDecodingMode = hardwareDecodingMode === "hardware" ? "software" : "hardware";
    setHardwareDecodingModeValue(nextMode);
    hardwareDecodingModeRef.current = nextMode;
    persistPlaybackSettings({ hwdecMode: nextMode });
    if (!media) {
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_hwdec", { mode: nextMode })
      .then(applyCommandSnapshot)
      .catch((error: unknown) => {
        setHardwareDecodingModeValue(hardwareDecodingMode);
        hardwareDecodingModeRef.current = hardwareDecodingMode;
        onError(error);
      });
  }

  function setVideoFillMode(enabled: boolean) {
    const previousValue = isVideoFillEnabled;
    setIsVideoFillEnabled(enabled);
    persistPlaybackSettings({ videoFill: enabled });
    if (!media) {
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_video_fill", { enabled })
      .then(applyCommandSnapshot)
      .catch((error: unknown) => {
        setIsVideoFillEnabled(previousValue);
        onError(error);
      });
  }

  function setLoopMode(mode: LoopMode) {
    setLoopModeValue(mode);
    persistPlaybackSettings({ loopMode: mode });
  }

  return {
    toggleTimeDisplayMode,
    setVolume,
    toggleMute,
    setPlaybackSpeed,
    toggleHardwareDecoding,
    setVideoFillMode,
    setLoopMode,
  };
}
