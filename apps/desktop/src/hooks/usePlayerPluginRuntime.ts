import type {
  AppVersionInfo,
  LoopMode,
  PluginActionDefinition,
  PluginMediaOpenInput,
  PluginMediaOpenResult,
  ThemePluginSummary,
  TimeDisplayMode,
} from "../app/types";
import type { AppLocale } from "../i18n";
import { usePluginActions } from "./usePluginActions";
import { usePluginRuntimeCommands } from "./usePluginRuntimeCommands";
import { usePluginRuntimeHost } from "./usePluginRuntimeHost";
import type { PluginRuntimeCommandContext } from "./pluginRuntimeCommands/types";

type PluginCaptureAction = (plugin: ThemePluginSummary, action: PluginActionDefinition) => Promise<void>;

type UsePlayerPluginRuntimeOptions = PluginRuntimeCommandContext & {
  activePluginView: Parameters<typeof usePluginRuntimeHost>[0]["activePluginView"];
  appVersion: AppVersionInfo | null;
  locale: AppLocale;
  currentTime: number;
  duration: number;
  isPlaying: boolean;
  playbackSpeed: number;
  volumeLevel: number;
  loopMode: LoopMode;
  timeDisplayMode: TimeDisplayMode;
  onError: (error: unknown) => void;
  onRuntimeLog: (pluginId: string, level: "info" | "warning" | "error", message: string) => void;
  capturePluginScreenshot: PluginCaptureAction;
  startPluginRecording: PluginCaptureAction;
  stopPluginRecording: PluginCaptureAction;
  togglePluginRecording: PluginCaptureAction;
};

export function usePlayerPluginRuntime({
  activePluginView,
  appVersion,
  locale,
  currentTime,
  duration,
  isPlaying,
  playbackSpeed,
  volumeLevel,
  loopMode,
  timeDisplayMode,
  onError,
  onRuntimeLog,
  capturePluginScreenshot,
  startPluginRecording,
  stopPluginRecording,
  togglePluginRecording,
  ...commandContext
}: UsePlayerPluginRuntimeOptions) {
  const {
    appearanceState,
    pluginViewFrameRef,
    media,
    queue,
    currentIndex,
    openNativeMediaFiles,
    openRuntimeStream,
    openNetworkStreamDialog,
    togglePlayback,
    stopPlayback,
    restartPlayback,
    togglePlaylist,
    toggleTrackPanel,
    toggleLoopPanel,
    toggleSpeedPanel,
    toggleFullscreen,
    toggleAlwaysOnTop,
    openSettingsDialog,
  } = commandContext;
  const runtimeRefreshKey = appearanceState?.plugins.map((plugin) => `${plugin.id}:${plugin.enabled}:${plugin.runtime}:${plugin.version}`).join("|") ?? "";
  const executePluginRuntimeCommand = usePluginRuntimeCommands({ ...commandContext, duration });
  const { broadcastPluginRuntimeEvent, executePluginRuntimeAction, runMediaOpeningHooks } = usePluginRuntimeHost({
    activePluginView,
    plugins: appearanceState?.plugins ?? [],
    runtimeRefreshKey,
    commandHandler: executePluginRuntimeCommand,
    hostState: () => ({
      version: appVersion,
      locale,
      media,
      queue,
      currentIndex,
      playback: {
        playing: isPlaying,
        position: currentTime,
        duration,
        speed: playbackSpeed,
        volume: Math.round(volumeLevel * 100),
        loopMode,
        timeDisplayMode,
      },
    }),
    pluginViewFrameRef,
    onRuntimeLog,
  });
  const pluginActions = usePluginActions({
    appearanceState,
    media,
    onError,
    executePluginRuntimeAction,
    openNativeMediaFiles,
    openNetworkStreamDialog,
    openRuntimeStream,
    capturePluginScreenshot,
    startPluginRecording,
    stopPluginRecording,
    togglePluginRecording,
    togglePlayback,
    stopPlayback,
    restartPlayback,
    togglePlaylist,
    toggleTrackPanel,
    toggleLoopPanel,
    toggleSpeedPanel,
    toggleFullscreen,
    toggleAlwaysOnTop,
    openSettingsDialog,
  });

  return {
    ...pluginActions,
    broadcastPluginRuntimeEvent,
    runMediaOpeningHooks: (input: PluginMediaOpenInput): Promise<PluginMediaOpenResult> => runMediaOpeningHooks(input),
  };
}
