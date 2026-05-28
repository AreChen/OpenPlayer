import { invoke } from "@tauri-apps/api/core";
import { runtimeBooleanArg, runtimeNumberArg, runtimeStringArg } from "../../app/pluginRuntime";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginAudioRuntimeCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
  permissions,
  pluginId,
) => {
  switch (command) {
    case "audio.extractClip": {
      if (!context.media) {
        throw new Error("audio.extractClip requires loaded media");
      }
      if (!permissions.has("audio.extract")) {
        throw new Error("plugin runtime command requires audio.extract");
      }
      return await invoke("mpv_embed_extract_audio_clip", {
        pluginId,
        start: runtimeNumberArg(record, "start"),
        duration: runtimeNumberArg(record, "duration"),
        sampleRate: runtimeNumberArg(record, "sampleRate"),
        channels: runtimeStringArg(record, "channels"),
        includeBase64: runtimeBooleanArg(record, "includeBase64"),
      });
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
