import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "../types";

export const handlePluginPlayerPanelCommand: PluginRuntimeCommandHandler = async (context, command) => {
  switch (command) {
    case "player.togglePlaylist":
      context.togglePlaylist();
      return null;
    case "player.toggleTracks":
      context.toggleTrackPanel();
      return null;
    case "player.toggleLoop":
      context.toggleLoopPanel();
      return null;
    case "player.toggleSpeed":
      context.toggleSpeedPanel();
      return null;
    case "window.toggleFullscreen":
      context.toggleFullscreen();
      return null;
    case "window.toggleAlwaysOnTop":
      context.toggleAlwaysOnTop();
      return null;
    case "app.openSettings":
      context.openSettingsDialog();
      return null;
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
