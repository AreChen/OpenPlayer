import {
  isPluginRuntimeActionCommand,
  pluginActionCommandRequiresMedia,
  pluginActionStringArg,
} from "../app/pluginRuntime";
import type {
  AppearanceState,
  MediaItem,
  PluginActionDefinition,
  PluginActionInstance,
  PluginActionPlacement,
  ThemePluginSummary,
} from "../app/types";

type PluginCaptureAction = (plugin: ThemePluginSummary, action: PluginActionDefinition) => Promise<void>;

type UsePluginActionsOptions = {
  appearanceState: AppearanceState | null;
  media: MediaItem | null;
  onError: (error: unknown) => void;
  executePluginRuntimeAction: (instance: PluginActionInstance) => Promise<void>;
  openNativeMediaFiles: () => void;
  openNetworkStreamDialog: () => void;
  openRuntimeStream: (url: string, name?: string | null) => Promise<void>;
  capturePluginScreenshot: PluginCaptureAction;
  startPluginRecording: PluginCaptureAction;
  stopPluginRecording: PluginCaptureAction;
  togglePluginRecording: PluginCaptureAction;
  togglePlayback: () => void;
  stopPlayback: () => void;
  restartPlayback: () => void;
  togglePlaylist: () => void;
  toggleTrackPanel: () => void;
  toggleLoopPanel: () => void;
  toggleSpeedPanel: () => void;
  toggleFullscreen: () => void;
  toggleAlwaysOnTop: () => void;
  openSettingsDialog: () => void;
};

export function usePluginActions({
  appearanceState,
  media,
  onError,
  executePluginRuntimeAction,
  openNativeMediaFiles,
  openNetworkStreamDialog,
  openRuntimeStream,
  capturePluginScreenshot,
  startPluginRecording,
  stopPluginRecording,
  togglePluginRecording,
  togglePlayback,
  stopPlayback,
  restartPlayback,
  togglePlaylist,
  toggleTrackPanel,
  toggleLoopPanel,
  toggleSpeedPanel,
  toggleFullscreen,
  toggleAlwaysOnTop,
  openSettingsDialog,
}: UsePluginActionsOptions) {
  const pluginActionInstances: PluginActionInstance[] = (appearanceState?.plugins ?? [])
    .filter((plugin) => plugin.enabled)
    .flatMap((plugin) => plugin.actions.map((action) => ({ plugin, action })));

  function actionsForPlacement(placement: PluginActionPlacement) {
    return pluginActionInstances.filter(({ action }) => action.placement === placement);
  }

  function isPluginActionDisabled(action: PluginActionDefinition) {
    return (action.requiresMedia || pluginActionCommandRequiresMedia(action.command)) && !media;
  }

  function executePluginAction({ plugin, action }: PluginActionInstance) {
    if (isPluginActionDisabled(action)) {
      return;
    }

    if (isPluginRuntimeActionCommand(action.command)) {
      executePluginRuntimeAction({ plugin, action }).catch(onError);
      return;
    }

    switch (action.command) {
      case "player.openMedia":
        openNativeMediaFiles();
        return;
      case "player.openStream":
        openPluginStream(action).catch(onError);
        return;
      case "player.openStreamDialog":
        openNetworkStreamDialog();
        return;
      case "player.captureScreenshot":
        capturePluginScreenshot(plugin, action).catch(onError);
        return;
      case "player.startRecording":
        startPluginRecording(plugin, action).catch(onError);
        return;
      case "player.stopRecording":
        stopPluginRecording(plugin, action).catch(onError);
        return;
      case "player.toggleRecording":
        togglePluginRecording(plugin, action).catch(onError);
        return;
      case "player.togglePlayback":
        togglePlayback();
        return;
      case "player.stop":
        stopPlayback();
        return;
      case "player.restart":
        restartPlayback();
        return;
      case "player.togglePlaylist":
        togglePlaylist();
        return;
      case "player.toggleTracks":
        toggleTrackPanel();
        return;
      case "player.toggleLoop":
        toggleLoopPanel();
        return;
      case "player.toggleSpeed":
        toggleSpeedPanel();
        return;
      case "window.toggleFullscreen":
        toggleFullscreen();
        return;
      case "window.toggleAlwaysOnTop":
        toggleAlwaysOnTop();
        return;
      case "app.openSettings":
        openSettingsDialog();
        return;
    }
  }

  async function openPluginStream(action: PluginActionDefinition) {
    const url = pluginActionStringArg(action, "url");
    if (!url) {
      throw new Error("plugin stream action is missing a url");
    }

    await openRuntimeStream(url, pluginActionStringArg(action, "name"));
  }

  return {
    pluginControlLeftActions: actionsForPlacement("controls.left"),
    pluginControlCenterActions: actionsForPlacement("controls.center"),
    pluginControlRightActions: actionsForPlacement("controls.right"),
    pluginContextMenuActions: actionsForPlacement("contextMenu"),
    pluginPlaylistActions: actionsForPlacement("playlist.actions"),
    isPluginActionDisabled,
    executePluginAction,
  };
}
