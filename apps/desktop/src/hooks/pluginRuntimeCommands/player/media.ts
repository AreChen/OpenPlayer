import { invoke } from "@tauri-apps/api/core";
import {
  normalizePluginLoadOptions,
  normalizePluginMediaSegment,
  normalizePluginMediaSegmentTimeline,
  runtimeBooleanArg,
  runtimeNumberArg,
  runtimeStringArg,
} from "../../../app/pluginRuntime";
import type { MpvSnapshot } from "../../../app/types";
import { focusOverlayWindow } from "../../../app/windowControls";
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
    case "player.segmentTimeline":
      return normalizePluginMediaSegmentTimeline(
        {
          start: runtimeNumberArg(record, "start") ?? undefined,
          end: runtimeNumberArg(record, "end") ?? undefined,
          duration: runtimeNumberArg(record, "duration") ?? undefined,
          overlap: runtimeNumberArg(record, "overlap") ?? undefined,
          maxSegments: runtimeNumberArg(record, "maxSegments") ?? undefined,
        },
        {
          media: context.media,
          position: context.displayTime,
          mediaDuration: context.duration,
        },
      );
    case "media.exportSegment": {
      if (!context.media) {
        throw new Error("media.exportSegment requires loaded media");
      }
      if (!permissions.has("media.export")) {
        throw new Error("plugin runtime command requires media.export");
      }
      const artifact = await invoke<{ path: string }>("mpv_embed_export_media_segment", {
        kind: runtimeStringArg(record, "kind"),
        format: runtimeStringArg(record, "format"),
        start: runtimeNumberArg(record, "start"),
        duration: runtimeNumberArg(record, "duration"),
        fileName: runtimeStringArg(record, "fileName"),
        directory: null,
      });
      if (runtimeBooleanArg(record, "openFolder") && artifact.path) {
        await invoke("window_reveal_path", { path: artifact.path });
        focusOverlayWindow();
      }
      return artifact;
    }
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
