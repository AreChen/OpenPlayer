import type { RefObject } from "react";
import type {
  AppearanceState,
  ContextMenuPosition,
  MediaItem,
  MediaPanelMode,
  MpvRecordingState,
  MpvSnapshot,
  SettingsSection,
} from "../../app/types";

export const PLUGIN_RUNTIME_COMMAND_NOT_HANDLED = Symbol("PLUGIN_RUNTIME_COMMAND_NOT_HANDLED");

export type PluginRuntimeCommandText = {
  dialog: {
    mediaFiles: string;
    subtitle: string;
  };
  status: {
    screenshotSaved: (directory: string | null, copiedToClipboard: boolean) => string;
    recordingStarted: string;
    recordingSaved: (directory: string | null) => string;
  };
};

export type PluginRuntimeCommandContext = {
  appearanceState: AppearanceState | null;
  pluginViewFrameRef: RefObject<HTMLIFrameElement | null>;
  t: PluginRuntimeCommandText;
  media: MediaItem | null;
  queue: MediaItem[];
  currentIndex: number | null;
  displayTime: number;
  duration: number;
  isPickerOpen: boolean;
  setIsPickerOpen: (isPickerOpen: boolean) => void;
  setSettingsSection: (section: SettingsSection) => void;
  setIsSettingsOpen: (isSettingsOpen: boolean) => void;
  setContextMenu: (contextMenu: ContextMenuPosition | null) => void;
  setMediaPanelMode: (mode: MediaPanelMode | null) => void;
  setIsPlaylistOpen: (isPlaylistOpen: boolean) => void;
  clearPlaylist: () => void;
  setRecordingState: (state: MpvRecordingState) => void;
  showCaptureFeedback: (icon: "camera" | "record" | "info", message: string) => void;
  openPluginView: (pluginId: string, viewId: string) => Promise<void>;
  closePluginView: () => void;
  openQueueIndex: (index: number) => Promise<void>;
  stopPlayback: () => void;
  replaceQueueWithMediaPaths: (paths: string[]) => Promise<void>;
  appendMediaPaths: (paths: string[]) => Promise<void>;
  openNativeMediaFiles: () => void;
  openRuntimeStream: (url: string, name?: string | null, loadOptions?: Record<string, string>) => Promise<void>;
  openNetworkStreamDialog: () => void;
  invalidatePendingSnapshots: () => void;
  clearPendingSeek: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  persistMediaPlaybackSettings: (path: string, settings: { subtitleTrackId?: number | null }) => void;
  seekTarget: (value: number) => number;
  seekTo: (value: number) => void;
  setVolume: (value: number, options?: { feedback?: boolean }) => void;
  setPlaybackSpeed: (speed: number) => void;
  setLoopMode: (mode: "off" | "one" | "all") => void;
  setVideoFillMode: (enabled: boolean) => void;
  setSubtitleDelay: (delay: number) => void;
  togglePlayback: () => void;
  restartPlayback: () => void;
  togglePlaylist: () => void;
  toggleTrackPanel: () => void;
  toggleLoopPanel: () => void;
  toggleSpeedPanel: () => void;
  toggleFullscreen: () => void;
  toggleAlwaysOnTop: () => void;
  openSettingsDialog: () => void;
};

export type PluginRuntimeCommandHandlerResult = Promise<unknown | typeof PLUGIN_RUNTIME_COMMAND_NOT_HANDLED>;

export type PluginRuntimeCommandHandler = (
  context: PluginRuntimeCommandContext,
  command: string,
  record: Record<string, unknown>,
  permissions: Set<string>,
  pluginId: string,
) => PluginRuntimeCommandHandlerResult;
