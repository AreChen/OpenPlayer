import type { AppLocale, AppStrings } from "../i18n";
import type {
  AppearanceState,
  LoopMode,
  MediaItem,
  MpvLoadOptions,
  MpvSnapshot,
  PlatformSupport,
  PlaybackHistoryEntry,
  PlayerPreferences,
  PluginMediaOpenInput,
  ThemeCatalogItem,
} from "../app/types";
import type { MutableRefObject } from "react";
import { useMediaIntakeActions } from "./useMediaIntakeActions";
import { useMediaQueueActions } from "./useMediaQueueActions";
import { useNetworkStreams } from "./useNetworkStreams";
import { usePluginView } from "./usePluginView";
import { useTrackActions } from "./useTrackActions";

type PreparedMediaOpen = {
  item: MediaItem;
  loadOptions: MpvLoadOptions;
};

type UsePlayerMediaDomainOptions = {
  platformSupport: PlatformSupport | null;
  t: AppStrings;
  locale: AppLocale;
  playerPreferences: PlayerPreferences;
  queue: MediaItem[];
  media: MediaItem | null;
  currentIndex: number | null;
  loopMode: LoopMode;
  handledEndedPathRef: MutableRefObject<string | null>;
  appearanceState: AppearanceState | null;
  activeTheme: ThemeCatalogItem | null;
  isPickerOpen: boolean;
  setIsPickerOpen: (isOpen: boolean) => void;
  setPlaybackError: (error: string | null) => void;
  setQueue: (queue: MediaItem[] | ((current: MediaItem[]) => MediaItem[])) => void;
  setCurrentIndex: (index: number | null) => void;
  setIsPlaylistOpen: (isOpen: boolean) => void;
  closeFloatingPlaybackMenus: () => void;
  closeContextMenu: () => void;
  setContextMenu: Parameters<typeof usePluginView>[0]["setContextMenu"];
  setMediaPanelMode: Parameters<typeof usePluginView>[0]["setMediaPanelMode"];
  setIsSettingsOpen: (isOpen: boolean) => void;
  setPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void;
  setSubtitleDelayValue: (delay: number) => void;
  preparePluginMediaOpen: (
    item: MediaItem,
    source: PluginMediaOpenInput["source"],
    loadOptions?: MpvLoadOptions,
  ) => Promise<PreparedMediaOpen>;
  openMpvPath: (path: string, loadOptions?: MpvLoadOptions) => Promise<void>;
  restartPlayback: (autoplay?: boolean) => void;
  updateAppearance: (request: Promise<AppearanceState>) => Promise<void>;
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  onError: (error: unknown) => void;
};

export function usePlayerMediaDomain({
  platformSupport,
  t,
  locale,
  playerPreferences,
  queue,
  media,
  currentIndex,
  loopMode,
  handledEndedPathRef,
  appearanceState,
  activeTheme,
  isPickerOpen,
  setIsPickerOpen,
  setPlaybackError,
  setQueue,
  setCurrentIndex,
  setIsPlaylistOpen,
  closeFloatingPlaybackMenus,
  closeContextMenu,
  setContextMenu,
  setMediaPanelMode,
  setIsSettingsOpen,
  setPlaybackHistory,
  setSubtitleDelayValue,
  preparePluginMediaOpen,
  openMpvPath,
  restartPlayback,
  updateAppearance,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  onError,
}: UsePlayerMediaDomainOptions) {
  const {
    networkStreamHistory,
    setNetworkStreamHistory,
    isNetworkStreamDialogOpen,
    networkStreamUrl,
    networkStreamError,
    setNetworkStreamUrl,
    setNetworkStreamError,
    openNetworkStreamDialog,
    closeNetworkStreamDialog,
    submitNetworkStream,
    openNetworkStreamHistoryEntry,
    clearNetworkStreamHistory,
    openRuntimeStream,
  } = useNetworkStreams({
    platformSupport,
    t,
    playerPreferences,
    preparePluginMediaOpen,
    openMpvPath,
    setPlaybackError,
    setQueue,
    setCurrentIndex,
    setIsPlaylistOpen,
    closeFloatingPlaybackMenus,
    closeContextMenu,
  });
  const { activePluginView, activePluginViewDocument, pluginViewFrameRef, openPluginView, closePluginView } = usePluginView({
    appearanceState,
    activeTheme,
    locale,
    setContextMenu,
    setIsPlaylistOpen,
    setMediaPanelMode,
    closeNetworkStreamDialog,
    setIsSettingsOpen,
  });
  const {
    replaceQueueWithMediaPaths,
    appendMediaPaths,
    openQueueIndex,
    chooseQueueItem,
    previousQueueIndex,
    nextQueueIndex,
    playPreviousQueueItem,
    playNextQueueItem,
    openHistoryEntry,
    clearPlaybackHistory,
    handlePlaybackEnd,
  } = useMediaQueueActions({
    platformSupport,
    t,
    queue,
    media,
    currentIndex,
    loopMode,
    handledEndedPathRef,
    setQueue,
    setCurrentIndex,
    setIsPlaylistOpen,
    setPlaybackHistory,
    setPlaybackError,
    preparePluginMediaOpen,
    openMpvPath,
    restartPlayback,
    onError,
  });
  const { openNativeMediaFiles, appendNativeMediaFiles, appendNativeMediaFolder, playDroppedPaths } = useMediaIntakeActions({
    platformSupport,
    t,
    isPickerOpen,
    setIsPickerOpen,
    setPlaybackError,
    updateAppearance,
    replaceQueueWithMediaPaths,
    appendMediaPaths,
    onError,
  });
  const { persistMediaPlaybackSettings, setSubtitleDelay, selectTrack, addExternalSubtitle } = useTrackActions({
    media,
    isPickerOpen,
    t,
    setIsPickerOpen,
    setSubtitleDelayValue,
    invalidatePendingSnapshots,
    applyCommandSnapshot,
    onError,
  });

  return {
    networkStreamHistory,
    setNetworkStreamHistory,
    isNetworkStreamDialogOpen,
    networkStreamUrl,
    networkStreamError,
    setNetworkStreamUrl,
    setNetworkStreamError,
    openNetworkStreamDialog,
    closeNetworkStreamDialog,
    submitNetworkStream,
    openNetworkStreamHistoryEntry,
    clearNetworkStreamHistory,
    openRuntimeStream,
    activePluginView,
    activePluginViewDocument,
    pluginViewFrameRef,
    openPluginView,
    closePluginView,
    replaceQueueWithMediaPaths,
    appendMediaPaths,
    openQueueIndex,
    chooseQueueItem,
    previousQueueIndex,
    nextQueueIndex,
    playPreviousQueueItem,
    playNextQueueItem,
    openHistoryEntry,
    clearPlaybackHistory,
    handlePlaybackEnd,
    openNativeMediaFiles,
    appendNativeMediaFiles,
    appendNativeMediaFolder,
    playDroppedPaths,
    persistMediaPlaybackSettings,
    setSubtitleDelay,
    selectTrack,
    addExternalSubtitle,
  };
}
