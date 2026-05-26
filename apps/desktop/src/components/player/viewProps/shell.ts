import type { PlayerAppViewProps, PlayerAppViewPropsContext } from "./types";

export function buildShellViewProps({
  appearanceStyle,
  media,
  isChromeHidden,
  shellHandlers,
  t,
  platformSupport,
  isAudioOnlyMedia,
  isPlaying,
  primaryAudioTrack,
  displayTime,
  duration,
  resizeFeedback,
  isDropActive,
  playbackError,
  volumeFeedback,
  alwaysOnTopFeedback,
  captureFeedback,
  recordingState,
  onWindowCommand,
  handleDragRegionDoubleClick,
  handleDragRegionPointerDown,
  handleDragRegionPointerMove,
  handleDragRegionPointerEnd,
  handleResizePointerEnter,
  handleResizePointerLeave,
  handleResizePointerDown,
  handleResizePointerMove,
  handleResizePointerEnd,
}: PlayerAppViewPropsContext): Pick<
  PlayerAppViewProps,
  | "appearanceStyle"
  | "mediaLoaded"
  | "isChromeHidden"
  | "shellHandlers"
  | "stageOverlaysProps"
  | "dragRegionHandlers"
  | "resizeRegionHandlers"
> {
  return {
    appearanceStyle,
    mediaLoaded: Boolean(media),
    isChromeHidden,
    shellHandlers,
    stageOverlaysProps: {
      t,
      media,
      platformSupport,
      isAudioOnlyMedia,
      isPlaying,
      primaryAudioTrack,
      displayTime,
      duration,
      resizeFeedback,
      isDropActive,
      playbackError,
      volumeFeedback,
      alwaysOnTopFeedback,
      captureFeedback,
      recordingState,
      onWindowCommand,
    },
    dragRegionHandlers: {
      onAuxClick: (event) => event.preventDefault(),
      onDragStart: (event) => event.preventDefault(),
      onDoubleClick: handleDragRegionDoubleClick,
      onPointerDown: handleDragRegionPointerDown,
      onPointerMove: handleDragRegionPointerMove,
      onPointerUp: handleDragRegionPointerEnd,
      onPointerCancel: handleDragRegionPointerEnd,
    },
    resizeRegionHandlers: {
      onPointerEnter: handleResizePointerEnter,
      onPointerLeave: handleResizePointerLeave,
      onPointerDown: handleResizePointerDown,
      onPointerMove: handleResizePointerMove,
      onPointerUp: handleResizePointerEnd,
      onPointerCancel: handleResizePointerEnd,
    },
  };
}
