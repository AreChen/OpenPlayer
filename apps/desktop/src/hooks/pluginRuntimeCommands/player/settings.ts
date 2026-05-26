import { clampPlaybackSpeed, clampSubtitleDelay, normalizeLoopMode } from "../../../app/playback";
import { runtimeBooleanArg, runtimeNumberArg, runtimeStringArg } from "../../../app/pluginRuntime";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "../types";
import { requireLoadedMedia } from "./shared";

export const handlePluginPlayerSettingsCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
) => {
  switch (command) {
    case "player.setVolume": {
      const volume = runtimeNumberArg(record, "volume");
      if (volume === null) {
        throw new Error("player.setVolume requires a volume");
      }
      context.setVolume(volume / 100, { feedback: runtimeBooleanArg(record, "feedback") });
      return { volume: Math.min(100, Math.max(0, volume)) };
    }
    case "player.setSpeed": {
      const speed = runtimeNumberArg(record, "speed");
      if (speed === null) {
        throw new Error("player.setSpeed requires a speed");
      }
      const nextSpeed = clampPlaybackSpeed(speed);
      context.setPlaybackSpeed(nextSpeed);
      return { speed: nextSpeed };
    }
    case "player.setLoopMode": {
      const mode = runtimeStringArg(record, "mode");
      const nextMode = normalizeLoopMode(mode);
      context.setLoopMode(nextMode);
      return { loopMode: nextMode };
    }
    case "player.setVideoFill": {
      const enabled = runtimeBooleanArg(record, "enabled");
      context.setVideoFillMode(enabled);
      return { videoFill: enabled };
    }
    case "player.setSubtitleDelay": {
      requireLoadedMedia(context, command);
      const delay = runtimeNumberArg(record, "delay");
      if (delay === null) {
        throw new Error("player.setSubtitleDelay requires a delay");
      }
      const nextDelay = clampSubtitleDelay(delay);
      context.setSubtitleDelay(nextDelay);
      return { subtitleDelay: nextDelay };
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
