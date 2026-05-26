import type { PlayerAppViewProps, PlayerAppViewPropsContext } from "./types";

export function buildPanelViewProps({
  t,
  locale,
  media,
  mediaPanelMode,
  playbackSpeed,
  setPlaybackSpeed,
  loopMode,
  loopModeOptions,
  setLoopMode,
  isAudioOnlyMedia,
  isVideoFillEnabled,
  audioTracks,
  videoTracks,
  subtitleTracks,
  subtitleDelay,
  subtitlePluginSettingGroups,
  isPickerOpen,
  systemFontFamilies,
  setVideoFillMode,
  selectTrack,
  setSubtitleDelay,
  setPluginSettingValue,
  choosePluginDirectory,
  openPluginDirectory,
  addExternalSubtitle,
  isPlaylistOpen,
  queueItems,
  currentIndex,
  playbackHistory,
  pluginPlaylistActions,
  isPluginActionDisabled,
  executePluginAction,
  appendNativeMediaFiles,
  appendNativeMediaFolder,
  chooseQueueItem,
  openHistoryEntry,
  clearPlaybackHistory,
}: PlayerAppViewPropsContext): Pick<
  PlayerAppViewProps,
  "speedPanelProps" | "loopPanelProps" | "tracksPanelProps" | "playlistDrawerProps"
> {
  return {
    speedPanelProps: mediaPanelMode === "speed" && media
      ? { t, playbackSpeed, onSetPlaybackSpeed: setPlaybackSpeed }
      : null,
    loopPanelProps: mediaPanelMode === "loop" && media
      ? { t, loopMode, loopModeOptions, onSetLoopMode: setLoopMode }
      : null,
    tracksPanelProps: mediaPanelMode === "tracks" && media
      ? {
          t,
          locale,
          isAudioOnlyMedia,
          isVideoFillEnabled,
          audioTracks,
          videoTracks,
          subtitleTracks,
          subtitleDelay,
          subtitlePluginSettingGroups,
          isPickerOpen,
          systemFontFamilies,
          onSetVideoFillMode: setVideoFillMode,
          onSelectTrack: selectTrack,
          onSetSubtitleDelay: setSubtitleDelay,
          onSetPluginSettingValue: setPluginSettingValue,
          onChoosePluginDirectory: choosePluginDirectory,
          onOpenPluginDirectory: openPluginDirectory,
          onAddExternalSubtitle: addExternalSubtitle,
        }
      : null,
    playlistDrawerProps: isPlaylistOpen
      ? {
          t,
          locale,
          queueItems,
          currentIndex,
          playbackHistory,
          currentMediaPath: media?.path ?? null,
          isPickerOpen,
          pluginPlaylistActions,
          isPluginActionDisabled,
          onExecutePluginAction: executePluginAction,
          onAppendNativeMediaFiles: appendNativeMediaFiles,
          onAppendNativeMediaFolder: appendNativeMediaFolder,
          onChooseQueueItem: chooseQueueItem,
          onOpenHistoryEntry: openHistoryEntry,
          onClearPlaybackHistory: clearPlaybackHistory,
        }
      : null,
  };
}
