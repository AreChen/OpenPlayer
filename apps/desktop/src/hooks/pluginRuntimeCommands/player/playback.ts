import { invoke } from "@tauri-apps/api/core";
import { runtimeNumberArg } from "../../../app/pluginRuntime";
import type { MpvSnapshot } from "../../../app/types";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "../types";
import { requireLoadedMedia, runSnapshotCommand } from "./shared";

export const handlePluginPlayerPlaybackCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
) => {
  switch (command) {
    case "player.play":
    case "player.pause":
      return runSnapshotCommand(context, command);
    case "player.seek": {
      requireLoadedMedia(context, command);
      const absolutePosition = runtimeNumberArg(record, "position");
      const delta = runtimeNumberArg(record, "delta");
      const target = context.seekTarget(absolutePosition ?? context.displayTime + (delta ?? 0));
      context.seekTo(target);
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_seek", { position: target });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "player.frameStep":
    case "player.frameBackStep":
      requireLoadedMedia(context, command);
      context.clearPendingSeek();
      return runSnapshotCommand(context, command);
    case "player.togglePlayback":
      context.togglePlayback();
      return null;
    case "player.stop":
      context.stopPlayback();
      return null;
    case "player.restart":
      context.restartPlayback();
      return null;
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
