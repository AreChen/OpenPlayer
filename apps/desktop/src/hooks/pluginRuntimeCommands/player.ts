import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "./types";
import { handlePluginPlayerCaptureCommand } from "./player/capture";
import { handlePluginPlayerMediaCommand } from "./player/media";
import { handlePluginPlayerPanelCommand } from "./player/panels";
import { handlePluginPlayerPlaybackCommand } from "./player/playback";
import { handlePluginPlayerSettingsCommand } from "./player/settings";
import { handlePluginPlayerTrackCommand } from "./player/tracks";

const playerCommandHandlers: PluginRuntimeCommandHandler[] = [
  handlePluginPlayerMediaCommand,
  handlePluginPlayerPlaybackCommand,
  handlePluginPlayerSettingsCommand,
  handlePluginPlayerTrackCommand,
  handlePluginPlayerCaptureCommand,
  handlePluginPlayerPanelCommand,
];

export const handlePluginPlayerRuntimeCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
  permissions,
  pluginId,
) => {
  for (const handler of playerCommandHandlers) {
    const result = await handler(context, command, record, permissions, pluginId);
    if (result !== PLUGIN_RUNTIME_COMMAND_NOT_HANDLED) {
      return result;
    }
  }

  return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
};
