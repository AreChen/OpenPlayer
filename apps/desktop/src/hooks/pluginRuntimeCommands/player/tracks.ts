import { invoke } from "@tauri-apps/api/core";
import { runtimeNumberArg, runtimeStringArg } from "../../../app/pluginRuntime";
import type { MpvSnapshot } from "../../../app/types";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "../types";
import { requireLoadedMedia } from "./shared";

export const handlePluginPlayerTrackCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
) => {
  if (command !== "player.selectTrack") {
    return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }

  requireLoadedMedia(context, command);
  const kind = runtimeStringArg(record, "kind");
  if (kind !== "audio" && kind !== "video" && kind !== "subtitle") {
    throw new Error("player.selectTrack requires kind audio, video, or subtitle");
  }
  const rawTrackId = record.trackId;
  const trackId = rawTrackId === null ? null : runtimeNumberArg(record, "trackId");
  if (rawTrackId !== null && trackId === null) {
    throw new Error("player.selectTrack requires numeric trackId or null");
  }

  context.invalidatePendingSnapshots();
  const snapshot = await invoke<MpvSnapshot>("mpv_embed_select_track", { kind, trackId });
  context.applyCommandSnapshot(snapshot);
  if (kind === "subtitle" && context.media) {
    context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: trackId });
  }
  return snapshot;
};
