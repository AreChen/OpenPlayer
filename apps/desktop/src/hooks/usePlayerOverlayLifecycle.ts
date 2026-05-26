import { useEffect } from "react";
import { focusOverlayWindow } from "../app/windowControls";
import type {
  AppVersionInfo,
  AppearanceState,
  ContextMenuPosition,
  LoopMode,
  MediaItem,
  MpvSnapshot,
  NetworkStreamHistoryEntry,
  PlatformSupport,
  PlaybackHistoryEntry,
  PlaybackSettings,
  PlayerPreferences,
  SettingsSection,
  ShellPreviewFormatInfo,
  ShortcutAction,
  ShortcutBindings,
  ShortcutDefinition,
} from "../app/types";
import { useBackendStateSync } from "./useBackendStateSync";
import { useLoopModeSync } from "./useLoopModeSync";
import { useMediaDropTarget } from "./useMediaDropTarget";
import { useReleaseUpdates } from "./useReleaseUpdates";
import { useShortcutBridge } from "./useShortcutBridge";

type UsePlayerOverlayLifecycleOptions = {
  media: MediaItem | null;
  loadedMediaPath: string | null;
  loopMode: LoopMode;
  appVersion: AppVersionInfo | null;
  platformSupport: PlatformSupport | null;
  isChromePinned: boolean;
  contextMenu: ContextMenuPosition | null;
  isSettingsOpen: boolean;
  settingsSection: SettingsSection;
  isNetworkStreamDialogOpen: boolean;
  recordingShortcutAction: ShortcutAction | null;
  shortcutBindings: ShortcutBindings;
  shortcutDefinitions: ShortcutDefinition[];
  recordUserActivity: () => void;
  recordShortcutActivity: (action: ShortcutAction) => void;
  performShortcutAction: (action: ShortcutAction) => void;
  assignShortcut: (action: ShortcutAction, shortcut: string | null) => void;
  setRecordingShortcutAction: (action: ShortcutAction | null) => void;
  setContextMenu: (position: ContextMenuPosition | null) => void;
  setIsSettingsOpen: (isOpen: boolean) => void;
  closeNetworkStreamDialog: () => void;
  playDroppedPaths: (paths: string[]) => Promise<void>;
  clearWindowFrameInteraction: () => void;
  setPlatformSupport: (support: PlatformSupport) => void;
  setPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void;
  setNetworkStreamHistory: (entries: NetworkStreamHistoryEntry[]) => void;
  setAppearanceState: (state: AppearanceState) => void;
  setPlayerPreferences: (preferences: PlayerPreferences) => void;
  applyPlaybackSettingsFromStore: (settings: PlaybackSettings) => void;
  setAppVersion: (version: AppVersionInfo) => void;
  setIsAlwaysOnTop: (enabled: boolean) => void;
  loadShellPreviewFormats: (formats: ShellPreviewFormatInfo[], selectedExtensions: string[]) => void;
  setSystemFontFamilies: (fonts: string[]) => void;
  replaceQueueWithMediaPaths: (paths: string[]) => Promise<void>;
  openExternalUrl: (url: string) => void;
  applySnapshot: (snapshot: MpvSnapshot) => void;
  onError: (error: unknown) => void;
};

export function usePlayerOverlayLifecycle({
  media,
  loadedMediaPath,
  loopMode,
  appVersion,
  platformSupport,
  isChromePinned,
  contextMenu,
  isSettingsOpen,
  settingsSection,
  isNetworkStreamDialogOpen,
  recordingShortcutAction,
  shortcutBindings,
  shortcutDefinitions,
  recordUserActivity,
  recordShortcutActivity,
  performShortcutAction,
  assignShortcut,
  setRecordingShortcutAction,
  setContextMenu,
  setIsSettingsOpen,
  closeNetworkStreamDialog,
  playDroppedPaths,
  clearWindowFrameInteraction,
  setPlatformSupport,
  setPlaybackHistory,
  setNetworkStreamHistory,
  setAppearanceState,
  setPlayerPreferences,
  applyPlaybackSettingsFromStore,
  setAppVersion,
  setIsAlwaysOnTop,
  loadShellPreviewFormats,
  setSystemFontFamilies,
  replaceQueueWithMediaPaths,
  openExternalUrl,
  applySnapshot,
  onError,
}: UsePlayerOverlayLifecycleOptions) {
  useShortcutBridge({
    contextMenu,
    isSettingsOpen,
    isNetworkStreamDialogOpen,
    recordingShortcutAction,
    shortcutBindings,
    shortcutDefinitions,
    onRecordUserActivity: recordUserActivity,
    onRecordShortcutActivity: recordShortcutActivity,
    onPerformShortcutAction: performShortcutAction,
    onAssignShortcut: assignShortcut,
    onCancelRecordingShortcut: () => setRecordingShortcutAction(null),
    onCloseContextMenu: () => setContextMenu(null),
    onCloseSettings: () => setIsSettingsOpen(false),
    onCloseNetworkStreamDialog: closeNetworkStreamDialog,
  });
  const { isDropActive } = useMediaDropTarget({
    onDropPaths: (paths) => {
      playDroppedPaths(paths).catch(onError);
    },
  });
  useBackendStateSync({
    onPlatformSupport: setPlatformSupport,
    onPlaybackHistory: setPlaybackHistory,
    onNetworkStreamHistory: setNetworkStreamHistory,
    onAppearanceState: setAppearanceState,
    onPlayerPreferences: setPlayerPreferences,
    onPlaybackSettings: applyPlaybackSettingsFromStore,
    onAppVersion: setAppVersion,
    onAlwaysOnTop: setIsAlwaysOnTop,
    onShellPreviewFormats: loadShellPreviewFormats,
    onSystemFontFamilies: setSystemFontFamilies,
    onStartupMediaPaths: (paths) => {
      replaceQueueWithMediaPaths(paths).catch(onError);
    },
  });
  const { updateState, checkForUpdates, openUpdateDownload } = useReleaseUpdates({
    appVersion,
    platformSupport,
    onSetAppVersion: setAppVersion,
    onOpenExternalUrl: openExternalUrl,
    onCheckSettled: focusOverlayWindow,
  });
  useLoopModeSync({
    media,
    loadedMediaPath,
    loopMode,
    applySnapshot,
    onError,
  });

  useEffect(() => {
    return () => {
      clearWindowFrameInteraction();
    };
  }, [clearWindowFrameInteraction, media?.id, isChromePinned]);

  useEffect(() => {
    if (isSettingsOpen && settingsSection === "about" && updateState.status === "idle") {
      checkForUpdates();
    }
  }, [checkForUpdates, isSettingsOpen, settingsSection, updateState.status]);

  return {
    isDropActive,
    updateState,
    checkForUpdates,
    openUpdateDownload,
  };
}
