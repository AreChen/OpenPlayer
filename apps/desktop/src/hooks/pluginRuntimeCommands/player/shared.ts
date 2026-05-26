import { invoke } from "@tauri-apps/api/core";
import type { MpvSnapshot } from "../../../app/types";
import type { PluginRuntimeCommandContext } from "../types";

export function requireLoadedMedia(context: PluginRuntimeCommandContext, command: string) {
  if (!context.media) {
    throw new Error(`${command} requires loaded media`);
  }
}

export async function runSnapshotCommand(context: PluginRuntimeCommandContext, command: string) {
  requireLoadedMedia(context, command);
  context.invalidatePendingSnapshots();
  const snapshot = await invoke<MpvSnapshot>(commandToMpvCommand(command));
  context.applyCommandSnapshot(snapshot);
  return snapshot;
}

function commandToMpvCommand(command: string) {
  switch (command) {
    case "player.play":
      return "mpv_embed_play";
    case "player.pause":
      return "mpv_embed_pause";
    case "player.frameStep":
      return "mpv_embed_frame_step";
    case "player.frameBackStep":
      return "mpv_embed_frame_back_step";
    default:
      throw new Error(`unsupported snapshot command: ${command}`);
  }
}
