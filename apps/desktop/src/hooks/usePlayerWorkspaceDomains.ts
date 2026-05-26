import type { MediaItem } from "../app/types";
import { usePlayerMediaDomain } from "./usePlayerMediaDomain";
import type { usePlayerOverlayFoundation } from "./usePlayerOverlayFoundation";
import type { usePlayerOverlayState } from "./usePlayerOverlayState";
import type { usePlayerPlaybackCoordinator } from "./usePlayerPlaybackCoordinator";
import { usePlayerSettingsDomain } from "./usePlayerSettingsDomain";
import { usePluginCaptureActions } from "./usePluginCaptureActions";
import { useWindowActions } from "./useWindowActions";

type PlayerOverlayFoundation = ReturnType<typeof usePlayerOverlayFoundation>;
type PlayerOverlayState = ReturnType<typeof usePlayerOverlayState>;
type PlayerPlaybackCoordinator = ReturnType<typeof usePlayerPlaybackCoordinator>;

type UsePlayerWorkspaceDomainsOptions = {
  media: MediaItem | null;
  state: PlayerOverlayState;
  foundation: PlayerOverlayFoundation;
  playback: PlayerPlaybackCoordinator;
};

export function usePlayerWorkspaceDomains({
  media,
  state,
  foundation,
  playback,
}: UsePlayerWorkspaceDomainsOptions) {
  const windowActions = useWindowActions({
    media,
    setIsAlwaysOnTop: state.setIsAlwaysOnTop,
    showAlwaysOnTopFeedback: foundation.showAlwaysOnTopFeedback,
    onError: foundation.reportPlaybackError,
  });
  const captureActions = usePluginCaptureActions({
    t: foundation.t,
    setRecordingState: state.setRecordingState,
    showCaptureFeedback: foundation.showCaptureFeedback,
  });
  const settings = usePlayerSettingsDomain({
    appearanceState: state.appearanceState,
    setAppearanceState: state.setAppearanceState,
    setPlayerPreferences: state.setPlayerPreferences,
    media,
    isPickerOpen: state.isPickerOpen,
    setIsPickerOpen: state.setIsPickerOpen,
    locale: foundation.locale,
    t: foundation.t,
    setContextMenu: foundation.setContextMenu,
    setMediaPanelMode: foundation.setMediaPanelMode,
    setRecordingShortcutAction: foundation.setRecordingShortcutAction,
    onError: foundation.reportPlaybackError,
    applyCommandSnapshot: playback.applyCommandSnapshot,
  });
  const mediaDomain = usePlayerMediaDomain({
    platformSupport: state.platformSupport,
    t: foundation.t,
    locale: foundation.locale,
    playerPreferences: state.playerPreferences,
    queue: state.queue,
    media,
    currentIndex: state.currentIndex,
    loopMode: state.loopMode,
    handledEndedPathRef: state.handledEndedPathRef,
    appearanceState: state.appearanceState,
    activeTheme: settings.activeTheme,
    isPickerOpen: state.isPickerOpen,
    setIsPickerOpen: state.setIsPickerOpen,
    setPlaybackError: foundation.setPlaybackError,
    setQueue: state.setQueue,
    setCurrentIndex: state.setCurrentIndex,
    setIsPlaylistOpen: foundation.setIsPlaylistOpen,
    closeFloatingPlaybackMenus: foundation.closeFloatingPlaybackMenus,
    closeContextMenu: foundation.closeContextMenu,
    setContextMenu: foundation.setContextMenu,
    setMediaPanelMode: foundation.setMediaPanelMode,
    setIsSettingsOpen: settings.setIsSettingsOpen,
    setPlaybackHistory: state.setPlaybackHistory,
    setSubtitleDelayValue: state.setSubtitleDelayValue,
    preparePluginMediaOpen: playback.preparePluginMediaOpen,
    openMpvPath: playback.openMpvPath,
    restartPlayback: playback.restartPlayback,
    updateAppearance: settings.updateAppearance,
    invalidatePendingSnapshots: playback.invalidatePendingSnapshots,
    applyCommandSnapshot: playback.applyCommandSnapshot,
    onError: foundation.reportPlaybackError,
  });

  return {
    windowActions,
    captureActions,
    settings,
    mediaDomain,
  };
}
