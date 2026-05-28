import { invoke } from "@tauri-apps/api/core";
import {
  normalizePluginLoadOptions,
  normalizePluginMediaSegment,
  runtimeNumberArg,
  runtimeStringArg,
} from "../../../app/pluginRuntime";
import type { MpvSnapshot } from "../../../app/types";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "../types";

export const handlePluginPlayerMediaCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
  permissions,
) => {
  switch (command) {
    case "player.currentMedia":
      return { media: context.media, queue: context.queue, currentIndex: context.currentIndex };
    case "player.snapshot":
      return invoke<MpvSnapshot | null>("mpv_embed_snapshot");
    case "player.currentSegment":
      return normalizePluginMediaSegment(
        {
          start: runtimeNumberArg(record, "start") ?? undefined,
          before: runtimeNumberArg(record, "before") ?? undefined,
          duration: runtimeNumberArg(record, "duration") ?? undefined,
        },
        {
          media: context.media,
          position: context.displayTime,
          mediaDuration: context.duration,
        },
      );
    case "player.openMedia":
      context.openNativeMediaFiles();
      return null;
    case "player.openStream": {
      if (!permissions.has("media.openStream")) {
        throw new Error("plugin runtime command requires media.openStream");
      }
      const url = runtimeStringArg(record, "url");
      if (!url) {
        throw new Error("plugin runtime stream command is missing a url");
      }
      await context.openRuntimeStream(
        url,
        runtimeStringArg(record, "name"),
        permissions.has("mpv.loadOptions") ? normalizePluginLoadOptions(record.loadOptions) : {},
      );
      return { path: url };
    }
    case "player.openStreamDialog":
      if (!permissions.has("media.openStream")) {
        throw new Error("plugin runtime command requires media.openStream");
      }
      context.openNetworkStreamDialog();
      return null;
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
