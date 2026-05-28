import { invoke } from "@tauri-apps/api/core";
import { INACTIVE_RECORDING_STATE } from "../../../app/constants";
import { parentDirectoryFromPath } from "../../../app/media";
import { runtimeBooleanArg, runtimeStringArg } from "../../../app/pluginRuntime";
import type { MpvCaptureArtifact, MpvFrameCaptureArtifact, MpvRecordingState } from "../../../app/types";
import { focusOverlayWindow } from "../../../app/windowControls";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
  type PluginRuntimeCommandContext,
} from "../types";

export const handlePluginPlayerCaptureCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
  permissions,
  pluginId,
) => {
  switch (command) {
    case "player.captureScreenshot": {
      requireCapturePermission(permissions);
      const artifact = await invoke<MpvCaptureArtifact>("mpv_embed_capture_screenshot", {
        format: runtimeStringArg(record, "format"),
        directory: null,
      });
      if (runtimeBooleanArg(record, "openFolder")) {
        await invoke("window_reveal_path", { path: artifact.path });
      }
      context.showCaptureFeedback(
        "camera",
        context.t.status.screenshotSaved(parentDirectoryFromPath(artifact.path), artifact.copiedToClipboard),
      );
      focusOverlayWindow();
      return artifact;
    }
    case "capture.frame": {
      requireCapturePermission(permissions);
      return await invoke<MpvFrameCaptureArtifact>("mpv_embed_capture_plugin_frame", {
        pluginId,
        format: runtimeStringArg(record, "format"),
        includeBase64: runtimeBooleanArg(record, "includeBase64"),
      });
    }
    case "player.startRecording":
      requireCapturePermission(permissions);
      return startRecording(context, record);
    case "player.stopRecording":
      requireCapturePermission(permissions);
      return stopRecording(context, record);
    case "player.toggleRecording": {
      requireCapturePermission(permissions);
      const current = await invoke<MpvRecordingState>("mpv_embed_recording_state");
      return current.active ? stopRecording(context, record) : startRecording(context, record);
    }
    case "player.recordingState": {
      requireCapturePermission(permissions);
      const state = await invoke<MpvRecordingState>("mpv_embed_recording_state");
      context.setRecordingState(state);
      return state;
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};

function requireCapturePermission(permissions: Set<string>) {
  if (!permissions.has("mpv.capture")) {
    throw new Error("plugin runtime command requires mpv.capture");
  }
}

async function startRecording(context: PluginRuntimeCommandContext, record: Record<string, unknown>) {
  const state = await invoke<MpvRecordingState>("mpv_embed_start_recording", {
    format: runtimeStringArg(record, "format"),
    directory: null,
  });
  context.setRecordingState(state);
  context.showCaptureFeedback("record", context.t.status.recordingStarted);
  focusOverlayWindow();
  return state;
}

async function stopRecording(context: PluginRuntimeCommandContext, record: Record<string, unknown>) {
  try {
    const state = await invoke<MpvRecordingState>("mpv_embed_stop_recording");
    context.setRecordingState(state);
    if (runtimeBooleanArg(record, "openFolder") && state.path) {
      await invoke("window_reveal_path", { path: state.path });
    }
    context.showCaptureFeedback("record", context.t.status.recordingSaved(parentDirectoryFromPath(state.path)));
    focusOverlayWindow();
    return state;
  } catch (error) {
    context.setRecordingState(INACTIVE_RECORDING_STATE);
    throw error;
  }
}
