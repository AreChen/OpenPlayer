import { invoke } from "@tauri-apps/api/core";
import { INACTIVE_RECORDING_STATE } from "../app/constants";
import { parentDirectoryFromPath } from "../app/media";
import {
  pluginActionBooleanArgWithSetting,
  pluginActionDirectoryArgWithSetting,
  pluginActionStringArgWithSetting,
} from "../app/pluginRuntime";
import { focusOverlayWindow } from "../app/windowControls";
import type { AppStrings } from "../i18n";
import type { CaptureFeedback, MpvCaptureArtifact, MpvRecordingState, PluginActionDefinition, ThemePluginSummary } from "../app/types";

type UsePluginCaptureActionsOptions = {
  t: AppStrings;
  setRecordingState: (state: MpvRecordingState) => void;
  showCaptureFeedback: (icon: CaptureFeedback["icon"], message: string) => void;
};

export function usePluginCaptureActions({ t, setRecordingState, showCaptureFeedback }: UsePluginCaptureActionsOptions) {
  async function capturePluginScreenshot(plugin: ThemePluginSummary, action: PluginActionDefinition) {
    const format = pluginActionStringArgWithSetting(plugin, action, "format", "formatSetting");
    const directory = pluginActionDirectoryArgWithSetting(plugin, action);
    const artifact = await invoke<MpvCaptureArtifact>("mpv_embed_capture_screenshot", { format, directory });
    if (pluginActionBooleanArgWithSetting(plugin, action, "openFolder", "openFolderSetting")) {
      await invoke("window_reveal_path", { path: artifact.path });
    }
    showCaptureFeedback("camera", t.status.screenshotSaved(parentDirectoryFromPath(artifact.path), artifact.copiedToClipboard));
    focusOverlayWindow();
  }

  async function startPluginRecording(plugin: ThemePluginSummary, action: PluginActionDefinition) {
    const format = pluginActionStringArgWithSetting(plugin, action, "format", "formatSetting");
    const directory = pluginActionDirectoryArgWithSetting(plugin, action);
    const state = await invoke<MpvRecordingState>("mpv_embed_start_recording", { format, directory });
    setRecordingState(state);
    showCaptureFeedback("record", t.status.recordingStarted);
    focusOverlayWindow();
  }

  async function stopPluginRecording(plugin: ThemePluginSummary, action: PluginActionDefinition) {
    try {
      const state = await invoke<MpvRecordingState>("mpv_embed_stop_recording");
      setRecordingState(state);
      if (pluginActionBooleanArgWithSetting(plugin, action, "openFolder", "openFolderSetting") && state.path) {
        await invoke("window_reveal_path", { path: state.path });
      }
      showCaptureFeedback("record", t.status.recordingSaved(parentDirectoryFromPath(state.path)));
      focusOverlayWindow();
    } catch (error) {
      setRecordingState(INACTIVE_RECORDING_STATE);
      throw error;
    }
  }

  async function togglePluginRecording(plugin: ThemePluginSummary, action: PluginActionDefinition) {
    const current = await invoke<MpvRecordingState>("mpv_embed_recording_state");
    if (current.active) {
      await stopPluginRecording(plugin, action);
    } else {
      await startPluginRecording(plugin, action);
    }
  }

  return {
    capturePluginScreenshot,
    startPluginRecording,
    stopPluginRecording,
    togglePluginRecording,
  };
}
