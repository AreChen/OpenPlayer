import type { PluginActionCommand } from "../types";

export function isPluginRuntimeActionCommand(command: string): command is `plugin.${string}` {
  return command.startsWith("plugin.");
}

export function pluginActionCommandRequiresMedia(command: PluginActionCommand) {
  if (isPluginRuntimeActionCommand(command)) {
    return false;
  }
  return [
    "player.captureScreenshot",
    "player.startRecording",
    "player.stopRecording",
    "player.toggleRecording",
    "player.togglePlayback",
    "player.stop",
    "player.restart",
    "player.toggleTracks",
    "player.toggleLoop",
    "player.toggleSpeed",
  ].includes(command);
}
