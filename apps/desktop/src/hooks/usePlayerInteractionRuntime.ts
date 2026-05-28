import { useEffect, useRef } from "react";
import type { MediaItem } from "../app/types";
import { usePlaybackShortcutActions } from "./usePlaybackShortcutActions";
import { usePlayerChromeInteractions } from "./usePlayerChromeInteractions";
import { usePlayerOverlayLifecycle } from "./usePlayerOverlayLifecycle";
import type { usePlayerOverlayFoundation } from "./usePlayerOverlayFoundation";
import type { usePlayerOverlayState } from "./usePlayerOverlayState";
import type { usePlayerPlaybackCoordinator } from "./usePlayerPlaybackCoordinator";
import { usePlayerPluginRuntime } from "./usePlayerPluginRuntime";
import type { usePlayerWorkspaceDomains } from "./usePlayerWorkspaceDomains";

type PlayerOverlayFoundation = ReturnType<typeof usePlayerOverlayFoundation>;
type PlayerOverlayState = ReturnType<typeof usePlayerOverlayState>;
type PlayerPlaybackCoordinator = ReturnType<typeof usePlayerPlaybackCoordinator>;
type PlayerWorkspaceDomains = ReturnType<typeof usePlayerWorkspaceDomains>;

type UsePlayerInteractionRuntimeOptions = {
  media: MediaItem | null;
  state: PlayerOverlayState;
  foundation: PlayerOverlayFoundation;
  playback: PlayerPlaybackCoordinator;
  workspace: PlayerWorkspaceDomains;
};

export function usePlayerInteractionRuntime({
  media,
  state,
  foundation,
  playback,
  workspace,
}: UsePlayerInteractionRuntimeOptions) {
  const { captureActions, mediaDomain, settings, windowActions } = workspace;
  const chrome = usePlayerChromeInteractions({
    media,
    mediaPanelMode: foundation.mediaPanelMode,
    isPlaylistOpen: foundation.isPlaylistOpen,
    isPickerOpen: state.isPickerOpen,
    playbackError: foundation.playbackError,
    contextMenu: foundation.contextMenu,
    isSettingsOpen: settings.isSettingsOpen,
    isNetworkStreamDialogOpen: mediaDomain.isNetworkStreamDialogOpen,
    activePluginView: mediaDomain.activePluginView,
    quietKeyboardControls: state.playerPreferences.quietKeyboardControls,
    clearResizeHoverFeedbackRef: state.clearResizeHoverFeedbackRef,
    platformOs: state.platformSupport?.os,
    recordingShortcutAction: foundation.recordingShortcutAction,
    volumeLevel: state.volumeLevel,
    openContextMenu: foundation.openContextMenu,
    closeContextMenu: foundation.closeContextMenu,
    closePluginView: mediaDomain.closePluginView,
    closeFloatingPlaybackMenus: foundation.closeFloatingPlaybackMenus,
    togglePlayback: playback.togglePlayback,
    setVolume: playback.setVolume,
  });
  const pluginRuntime = usePlayerPluginRuntime({
    appearanceState: state.appearanceState,
    activePluginView: mediaDomain.activePluginView,
    appVersion: state.appVersion,
    locale: foundation.locale,
    currentTime: state.currentTime,
    duration: state.duration,
    isPlaying: state.isPlaying,
    playbackSpeed: state.playbackSpeed,
    volumeLevel: state.volumeLevel,
    loopMode: state.loopMode,
    timeDisplayMode: state.timeDisplayMode,
    pluginViewFrameRef: mediaDomain.pluginViewFrameRef,
    t: foundation.t,
    media,
    queue: state.queue,
    currentIndex: state.currentIndex,
    displayTime: playback.displayTime,
    isPickerOpen: state.isPickerOpen,
    setIsPickerOpen: state.setIsPickerOpen,
    setSettingsSection: settings.setSettingsSection,
    setIsSettingsOpen: settings.setIsSettingsOpen,
    setContextMenu: foundation.setContextMenu,
    setMediaPanelMode: foundation.setMediaPanelMode,
    setIsPlaylistOpen: foundation.setIsPlaylistOpen,
    clearPlaylist: () => {
      state.setQueue([]);
      state.setCurrentIndex(null);
      foundation.setIsPlaylistOpen(false);
    },
    setRecordingState: state.setRecordingState,
    showCaptureFeedback: foundation.showCaptureFeedback,
    openPluginView: mediaDomain.openPluginView,
    closePluginView: mediaDomain.closePluginView,
    openQueueIndex: mediaDomain.openQueueIndex,
    stopPlayback: playback.stopPlayback,
    replaceQueueWithMediaPaths: mediaDomain.replaceQueueWithMediaPaths,
    appendMediaPaths: mediaDomain.appendMediaPaths,
    openNativeMediaFiles: mediaDomain.openNativeMediaFiles,
    openRuntimeStream: mediaDomain.openRuntimeStream,
    openNetworkStreamDialog: mediaDomain.openNetworkStreamDialog,
    invalidatePendingSnapshots: playback.invalidatePendingSnapshots,
    clearPendingSeek: playback.clearPendingSeek,
    applyCommandSnapshot: playback.applyCommandSnapshot,
    persistMediaPlaybackSettings: mediaDomain.persistMediaPlaybackSettings,
    seekTarget: playback.seekTarget,
    seekTo: playback.seekTo,
    setVolume: playback.setVolume,
    setPlaybackSpeed: playback.setPlaybackSpeed,
    setLoopMode: playback.setLoopMode,
    setVideoFillMode: playback.setVideoFillMode,
    setSubtitleDelay: mediaDomain.setSubtitleDelay,
    togglePlayback: playback.togglePlayback,
    restartPlayback: playback.restartPlayback,
    togglePlaylist: foundation.togglePlaylist,
    toggleTrackPanel: foundation.toggleTrackPanel,
    toggleLoopPanel: foundation.toggleLoopPanel,
    toggleSpeedPanel: foundation.toggleSpeedPanel,
    toggleFullscreen: windowActions.toggleFullscreen,
    toggleAlwaysOnTop: windowActions.toggleAlwaysOnTop,
    openSettingsDialog: settings.openSettingsDialog,
    onError: foundation.reportPlaybackError,
    onRuntimeLog: (pluginId, level, message) => {
      state.setPluginRuntimeLogs((logs) => [
        {
          id: `${Date.now()}:${pluginId}:${logs.length}`,
          pluginId,
          level,
          message,
          createdAtMs: Date.now(),
        },
        ...logs,
      ].slice(0, 100));
    },
    capturePluginScreenshot: captureActions.capturePluginScreenshot,
    startPluginRecording: captureActions.startPluginRecording,
    stopPluginRecording: captureActions.stopPluginRecording,
    togglePluginRecording: captureActions.togglePluginRecording,
  });
  const previousPluginViewRef = useRef<typeof mediaDomain.activePluginView>(null);
  useEffect(() => {
    const previous = previousPluginViewRef.current;
    const active = mediaDomain.activePluginView;
    if (previous && (!active || previous.pluginId !== active.pluginId || previous.viewId !== active.viewId)) {
      pluginRuntime.broadcastPluginRuntimeEvent("plugin.view.closed", previous);
    }
    if (active && (!previous || previous.pluginId !== active.pluginId || previous.viewId !== active.viewId)) {
      pluginRuntime.broadcastPluginRuntimeEvent("plugin.view.opened", active);
    }
    previousPluginViewRef.current = active;
  }, [mediaDomain.activePluginView, pluginRuntime]);
  playback.bindPlayerMpvSessionHandlers({
    applyStoredPluginMpvSettings: settings.applyStoredPluginMpvSettings,
    broadcastPluginRuntimeEvent: pluginRuntime.broadcastPluginRuntimeEvent,
    handlePlaybackEnd: mediaDomain.handlePlaybackEnd,
    openNativeMediaFiles: mediaDomain.openNativeMediaFiles,
    runMediaOpeningHooks: pluginRuntime.runMediaOpeningHooks,
  });
  const shortcutActions = usePlaybackShortcutActions({
    media,
    queueLength: state.queue.length,
    duration: state.duration,
    displayTime: playback.displayTime,
    volumeLevel: state.volumeLevel,
    openNativeMediaFiles: mediaDomain.openNativeMediaFiles,
    togglePlayback: playback.togglePlayback,
    restartPlayback: playback.restartPlayback,
    togglePlaylist: foundation.togglePlaylist,
    setVolume: playback.setVolume,
    toggleFullscreen: windowActions.toggleFullscreen,
    toggleAlwaysOnTop: windowActions.toggleAlwaysOnTop,
    openSettingsDialog: settings.openSettingsDialog,
    commitSeekTo: playback.commitSeekTo,
    invalidatePendingSnapshots: playback.invalidatePendingSnapshots,
    applyCommandSnapshot: playback.applyCommandSnapshot,
    clearPendingSeek: playback.clearPendingSeek,
    onError: foundation.reportPlaybackError,
  });
  const lifecycle = usePlayerOverlayLifecycle({
    media,
    loadedMediaPath: state.loadedMediaPath,
    loopMode: state.loopMode,
    appVersion: state.appVersion,
    platformSupport: state.platformSupport,
    isChromePinned: chrome.isChromePinned,
    contextMenu: foundation.contextMenu,
    isSettingsOpen: settings.isSettingsOpen,
    settingsSection: settings.settingsSection,
    isNetworkStreamDialogOpen: mediaDomain.isNetworkStreamDialogOpen,
    recordingShortcutAction: foundation.recordingShortcutAction,
    shortcutBindings: foundation.shortcutBindings,
    shortcutDefinitions: foundation.shortcutDefinitions,
    recordUserActivity: chrome.recordUserActivity,
    recordShortcutActivity: chrome.recordShortcutActivity,
    performShortcutAction: shortcutActions.performShortcutAction,
    assignShortcut: foundation.assignShortcut,
    setRecordingShortcutAction: foundation.setRecordingShortcutAction,
    setContextMenu: foundation.setContextMenu,
    setIsSettingsOpen: settings.setIsSettingsOpen,
    closeNetworkStreamDialog: mediaDomain.closeNetworkStreamDialog,
    playDroppedPaths: mediaDomain.playDroppedPaths,
    clearWindowFrameInteraction: chrome.clearWindowFrameInteraction,
    setPlatformSupport: state.setPlatformSupport,
    setPlaybackHistory: state.setPlaybackHistory,
    setNetworkStreamHistory: mediaDomain.setNetworkStreamHistory,
    setAppearanceState: state.setAppearanceState,
    setPlayerPreferences: state.setPlayerPreferences,
    applyPlaybackSettingsFromStore: foundation.applyPlaybackSettingsFromStore,
    setAppVersion: state.setAppVersion,
    setIsAlwaysOnTop: state.setIsAlwaysOnTop,
    loadShellPreviewFormats: settings.loadShellPreviewFormats,
    setSystemFontFamilies: state.setSystemFontFamilies,
    replaceQueueWithMediaPaths: mediaDomain.replaceQueueWithMediaPaths,
    openExternalUrl: windowActions.openExternalUrl,
    applySnapshot: playback.applySnapshot,
    onError: foundation.reportPlaybackError,
  });

  return {
    chrome,
    pluginRuntime,
    shortcutActions,
    lifecycle,
  };
}
