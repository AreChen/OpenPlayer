import { runtimeArgsRecord } from "../app/pluginRuntime";
import { handlePluginDataRuntimeCommand } from "./pluginRuntimeCommands/data";
import { handlePluginFilesystemRuntimeCommand } from "./pluginRuntimeCommands/filesystem";
import { handlePluginPlayerRuntimeCommand } from "./pluginRuntimeCommands/player";
import { handlePluginPlaylistRuntimeCommand } from "./pluginRuntimeCommands/playlist";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandContext,
  type PluginRuntimeCommandHandler,
} from "./pluginRuntimeCommands/types";
import { handlePluginUiRuntimeCommand } from "./pluginRuntimeCommands/ui";
import { handlePluginWallRuntimeCommand } from "./pluginRuntimeCommands/wall";

const pluginRuntimeCommandHandlers: PluginRuntimeCommandHandler[] = [
  handlePluginDataRuntimeCommand,
  handlePluginWallRuntimeCommand,
  handlePluginUiRuntimeCommand,
  handlePluginFilesystemRuntimeCommand,
  handlePluginPlaylistRuntimeCommand,
  handlePluginPlayerRuntimeCommand,
];

export function usePluginRuntimeCommands(context: PluginRuntimeCommandContext) {
  return async function executePluginRuntimeCommand(command: string, args: unknown, permissions: Set<string>, pluginId: string) {
    const record = runtimeArgsRecord(args);
    for (const handler of pluginRuntimeCommandHandlers) {
      const result = await handler(context, command, record, permissions, pluginId);
      if (result !== PLUGIN_RUNTIME_COMMAND_NOT_HANDLED) {
        return result;
      }
    }

    throw new Error(`unsupported plugin runtime command: ${command}`);
  };
}
