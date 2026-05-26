import { runtimeNumberArg } from "../../app/pluginRuntime";
import { pickPluginMediaPaths } from "./filesystem";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginPlaylistRuntimeCommand: PluginRuntimeCommandHandler = async (context, command, record, permissions) => {
  switch (command) {
    case "playlist.current":
      return { media: context.media, queue: context.queue, currentIndex: context.currentIndex };
    case "playlist.playIndex": {
      const index = runtimeNumberArg(record, "index");
      if (index === null || !Number.isInteger(index) || index < 0 || index >= context.queue.length) {
        throw new Error("playlist.playIndex requires a valid index");
      }
      await context.openQueueIndex(index);
      return null;
    }
    case "playlist.clear":
      context.stopPlayback();
      context.clearPlaylist();
      return null;
    case "playlist.openMediaFiles": {
      const paths = await pickPluginMediaPaths(context, permissions, true);
      await context.replaceQueueWithMediaPaths(paths);
      return { paths };
    }
    case "playlist.appendMediaFiles": {
      const paths = await pickPluginMediaPaths(context, permissions, true);
      await context.appendMediaPaths(paths);
      return { paths };
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
