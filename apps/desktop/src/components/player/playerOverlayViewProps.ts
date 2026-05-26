import { buildContextMenuItems } from "../../app/contextMenu";
import { buildPlaybackViewModel } from "../../app/playbackViewModel";
import type { MediaItem, MpvTrack, PluginActionInstance, TimeDisplayMode } from "../../app/types";
import { buildPlayerAppViewProps } from "./playerAppViewProps";
import type { PlayerAppViewProps, PlayerAppViewPropsContext } from "./viewProps/types";

type DerivedViewProps =
  | "audioTracks"
  | "canShowFrames"
  | "contextMenuItems"
  | "currentTimeToggleLabel"
  | "currentTransportLabel"
  | "durationTimeToggleLabel"
  | "durationTransportLabel"
  | "effectiveTimeDisplayMode"
  | "hardwareDecodingLabel"
  | "hardwareDecodingToggleLabel"
  | "isAudioOnlyMedia"
  | "isChromeHidden"
  | "isMuted"
  | "nextIndex"
  | "previousIndex"
  | "primaryAudioTrack"
  | "progress"
  | "progressRatio"
  | "queueItems"
  | "subtitlePluginSettingGroups"
  | "subtitleTracks"
  | "videoTracks"
  | "volumeMuteLabel";

type PlayerOverlayViewPropsInput = Omit<PlayerAppViewPropsContext, DerivedViewProps> & {
  queue: MediaItem[];
  loadedMediaPath: string | null;
  tracks: MpvTrack[];
  framesPerSecond: number;
  timeDisplayMode: TimeDisplayMode;
  isChromeVisible: boolean;
  isChromePinned: boolean;
  previousQueueIndex: () => number | null;
  nextQueueIndex: () => number | null;
  isAlwaysOnTop: boolean;
  pluginContextMenuActions: PluginActionInstance[];
  restartPlayback: () => void;
  toggleFullscreen: () => void;
  toggleAlwaysOnTop: () => void;
  openSettingsDialog: () => void;
  openCurrentFileLocation: () => void;
};

export function buildPlayerOverlayViewProps({
  queue,
  loadedMediaPath,
  tracks,
  framesPerSecond,
  timeDisplayMode,
  isChromeVisible,
  isChromePinned,
  previousQueueIndex,
  nextQueueIndex,
  isAlwaysOnTop,
  pluginContextMenuActions,
  restartPlayback,
  toggleFullscreen,
  toggleAlwaysOnTop,
  openSettingsDialog,
  openCurrentFileLocation,
  ...viewContext
}: PlayerOverlayViewPropsInput): PlayerAppViewProps {
  const previousIndex = previousQueueIndex();
  const nextIndex = nextQueueIndex();
  const playbackViewModel = buildPlaybackViewModel({
    t: viewContext.t,
    locale: viewContext.locale,
    queue,
    media: viewContext.media,
    loadedMediaPath,
    tracks,
    displayTime: viewContext.displayTime,
    duration: viewContext.duration,
    isPlaying: viewContext.isPlaying,
    framesPerSecond,
    timeDisplayMode,
    volumeLevel: viewContext.volumeLevel,
    hardwareDecodingMode: viewContext.hardwareDecodingMode,
    appearanceState: viewContext.appearanceState,
    isChromeVisible,
    isChromePinned,
  });
  const contextMenuItems = buildContextMenuItems({
    t: viewContext.t,
    locale: viewContext.locale,
    shortcutBindings: viewContext.shortcutBindings,
    isPickerOpen: viewContext.isPickerOpen,
    isMediaLoaded: Boolean(viewContext.media),
    isPlaying: viewContext.isPlaying,
    isAlwaysOnTop,
    pluginContextMenuActions,
    isPluginActionDisabled: viewContext.isPluginActionDisabled,
    onExecutePluginAction: viewContext.executePluginAction,
    onOpenNativeMediaFiles: viewContext.openNativeMediaFiles,
    onAppendNativeMediaFiles: viewContext.appendNativeMediaFiles,
    onAppendNativeMediaFolder: viewContext.appendNativeMediaFolder,
    onTogglePlayback: viewContext.togglePlayback,
    onStopPlayback: viewContext.stopPlayback,
    onRestartPlayback: restartPlayback,
    onOpenCurrentFileLocation: openCurrentFileLocation,
    onToggleFullscreen: toggleFullscreen,
    onToggleAlwaysOnTop: toggleAlwaysOnTop,
    onOpenSettingsDialog: openSettingsDialog,
    onCloseWindow: () => viewContext.onWindowCommand("window_close"),
  });

  return buildPlayerAppViewProps({
    ...viewContext,
    ...playbackViewModel,
    previousIndex,
    nextIndex,
    contextMenuItems,
  });
}
