import { snapEndOfMediaPosition } from "../app/playback";
import type { MediaItem } from "../app/types";
import { usePlaybackControlActions } from "./usePlaybackControlActions";
import { usePlayerMpvSession } from "./usePlayerMpvSession";
import type { usePlayerOverlayFoundation } from "./usePlayerOverlayFoundation";
import type { usePlayerOverlayState } from "./usePlayerOverlayState";

type PlayerOverlayFoundation = ReturnType<typeof usePlayerOverlayFoundation>;
type PlayerOverlayState = ReturnType<typeof usePlayerOverlayState>;

type UsePlayerPlaybackCoordinatorOptions = {
  media: MediaItem | null;
  state: PlayerOverlayState;
  foundation: PlayerOverlayFoundation;
};

export function usePlayerPlaybackCoordinator({
  media,
  state,
  foundation,
}: UsePlayerPlaybackCoordinatorOptions) {
  const session = usePlayerMpvSession({
    media,
    playerPreferences: state.playerPreferences,
    duration: state.duration,
    isPlaying: state.isPlaying,
    playbackSpeed: state.playbackSpeed,
    pendingSeekRef: state.pendingSeekRef,
    handledEndedPathRef: state.handledEndedPathRef,
    hardwareDecodingModeRef: state.hardwareDecodingModeRef,
    previousAudibleVolumeRef: state.previousAudibleVolumeRef,
    setDuration: state.setDuration,
    setIsPlaying: state.setIsPlaying,
    setFramesPerSecond: state.setFramesPerSecond,
    setPlaybackSpeedValue: state.setPlaybackSpeedValue,
    setHardwareDecodingModeValue: state.setHardwareDecodingModeValue,
    setIsVideoFillEnabled: state.setIsVideoFillEnabled,
    setSubtitleDelayValue: state.setSubtitleDelayValue,
    setTracks: state.setTracks,
    setLoadedMediaPath: state.setLoadedMediaPath,
    setVolumeLevel: state.setVolumeLevel,
    setCurrentTime: state.setCurrentTime,
    setCurrentIndex: state.setCurrentIndex,
    setMediaPanelMode: foundation.setMediaPanelMode,
    setPlaybackHistory: state.setPlaybackHistory,
    setRecordingState: state.setRecordingState,
    setPlaybackError: foundation.setPlaybackError,
    loadPlaybackSettings: foundation.loadPlaybackSettings,
    onError: foundation.reportPlaybackError,
  });
  const displayTime = snapEndOfMediaPosition(
    session.displayPosition,
    state.duration,
    state.isPlaying,
  );
  const controls = usePlaybackControlActions({
    media,
    duration: state.duration,
    displayTime,
    framesPerSecond: state.framesPerSecond,
    isPlaying: state.isPlaying,
    volumeLevel: state.volumeLevel,
    hardwareDecodingMode: state.hardwareDecodingMode,
    isVideoFillEnabled: state.isVideoFillEnabled,
    timeDisplayMode: state.timeDisplayMode,
    previousAudibleVolumeRef: state.previousAudibleVolumeRef,
    hardwareDecodingModeRef: state.hardwareDecodingModeRef,
    setVolumeLevel: state.setVolumeLevel,
    setPlaybackSpeedValue: state.setPlaybackSpeedValue,
    setHardwareDecodingModeValue: state.setHardwareDecodingModeValue,
    setIsVideoFillEnabled: state.setIsVideoFillEnabled,
    setTimeDisplayModeValue: state.setTimeDisplayModeValue,
    setLoopModeValue: state.setLoopModeValue,
    persistPlaybackSettings: foundation.persistPlaybackSettings,
    invalidatePendingSnapshots: session.invalidatePendingSnapshots,
    applyCommandSnapshot: session.applyCommandSnapshot,
    anchorDisplayClock: session.anchorDisplayClock,
    showVolumeFeedback: foundation.showVolumeFeedback,
    onError: foundation.reportPlaybackError,
  });

  return {
    ...session,
    displayTime,
    ...controls,
  };
}
