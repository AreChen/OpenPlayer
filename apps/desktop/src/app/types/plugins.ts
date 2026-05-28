import type { MpvLoadOptions } from "./media";
import type { IconName } from "./ui";

export type ThemePluginSummary = {
  id: string;
  name: string;
  version: string;
  apiVersion: string;
  minHostVersion: string | null;
  author: string | null;
  updateUrl: string | null;
  description: string | null;
  enabled: boolean;
  packageKind: "legacyManifest" | "manifestFile" | "directory" | "opplugin";
  installPath: string | null;
  installedAtMs: number | null;
  themeCount: number;
  runtime: "manifest" | "webviewJs" | "wasm";
  capabilityCount: number;
  settingCount: number;
  actionCount: number;
  permissions: string[];
  capabilities: PluginCapabilitySummary[];
  settings: PluginSettingDefinition[];
  actions: PluginActionDefinition[];
  views: PluginViewDefinition[];
};

export type PluginCapabilitySummary = {
  id: string;
  name: string;
  kind: PluginCapabilityKind;
  description: string | null;
  nameI18n: Record<string, string>;
  descriptionI18n: Record<string, string>;
  permissions: string[];
};

export type PluginCapabilityKind =
  | "subtitleStyle"
  | "capture"
  | "streamSource"
  | "mpvControl"
  | "aiTranscription"
  | "aiTranslation";
export type PluginSettingKind = "boolean" | "number" | "text" | "select" | "color" | "directory";
export type PluginSettingPlacement =
  | "pluginSettings"
  | "subtitleSettings"
  | "captureSettings"
  | "streamSettings"
  | "controls.left"
  | "controls.center"
  | "controls.right"
  | "contextMenu"
  | "overlay.status"
  | "playlist.actions";
export type PluginSettingValue = boolean | number | string;

export type PluginSettingOption = {
  value: string;
  label: string;
  labelI18n: Record<string, string>;
};

export type PluginSettingDefinition = {
  id: string;
  label: string;
  description: string | null;
  labelI18n: Record<string, string>;
  descriptionI18n: Record<string, string>;
  kind: PluginSettingKind;
  placement: PluginSettingPlacement;
  defaultValue: PluginSettingValue;
  value: PluginSettingValue;
  min: number | null;
  max: number | null;
  step: number | null;
  options: PluginSettingOption[];
  mpvProperty: string | null;
};

export type PluginActionPlacement =
  | "controls.left"
  | "controls.center"
  | "controls.right"
  | "contextMenu"
  | "overlay.status"
  | "playlist.actions";
export type PluginActionCommand =
  | "player.openMedia"
  | "player.openStream"
  | "player.openStreamDialog"
  | "player.captureScreenshot"
  | "player.startRecording"
  | "player.stopRecording"
  | "player.toggleRecording"
  | "player.togglePlayback"
  | "player.stop"
  | "player.restart"
  | "player.togglePlaylist"
  | "player.toggleTracks"
  | "player.toggleLoop"
  | "player.toggleSpeed"
  | "window.toggleFullscreen"
  | "window.toggleAlwaysOnTop"
  | "app.openSettings"
  | `plugin.${string}`;

export type PluginActionDefinition = {
  id: string;
  label: string;
  description: string | null;
  labelI18n: Record<string, string>;
  descriptionI18n: Record<string, string>;
  placement: PluginActionPlacement;
  command: PluginActionCommand;
  icon: IconName | null;
  requiresMedia: boolean;
  args: Record<string, unknown>;
};

export type PluginViewDefinition = {
  id: string;
  title: string;
  entry: string;
  description: string | null;
  presentation: PluginViewPresentation;
  frameOpacitySetting: string | null;
  titleI18n: Record<string, string>;
  descriptionI18n: Record<string, string>;
};

export type PluginViewPresentation = "overlay" | "sidePanel";

export type PluginActionInstance = {
  plugin: ThemePluginSummary;
  action: PluginActionDefinition;
};

export type PluginViewHtml = {
  pluginId: string;
  viewId: string;
  title: string;
  html: string;
};

export type PluginRuntimeSource = {
  pluginId: string;
  name: string;
  version: string;
  entry: string;
  script: string;
  permissions: string[];
  events: PluginRuntimeEventName[];
};

export type PluginRuntimeEventName =
  | "app.ready"
  | "media.opening"
  | "media.loaded"
  | "playback.snapshot"
  | "playback.started"
  | "playback.paused"
  | "playback.ended"
  | "playback.stopped"
  | "playback.seeked"
  | "playback.volumeChanged"
  | "playback.speedChanged"
  | "tracks.changed"
  | "theme.changed"
  | "window.fullscreenChanged"
  | "plugin.view.opened"
  | "plugin.view.closed";

export type PluginRuntimeLogLevel = "info" | "warning" | "error";

export type PluginRuntimeLogEntry = {
  id: string;
  pluginId: string;
  level: PluginRuntimeLogLevel;
  message: string;
  createdAtMs: number;
};

export type PluginRuntimeWorkerState = {
  pluginId: string;
  signature: string;
  worker: Worker;
  objectUrl: string;
  permissions: Set<string>;
  allowedEvents: Set<string>;
  eventSubscriptions: Set<string>;
  pendingHooks: Map<number, { resolve: (value: unknown) => void; reject: (error: Error) => void; timeout: number }>;
  nextHookId: number;
};

export type PluginMediaOpenInput = {
  path: string;
  name: string;
  source: "file" | "stream" | "history" | "playlist";
  loadOptions: MpvLoadOptions;
};

export type PluginMediaOpenResult = {
  path: string;
  name: string;
  loadOptions: MpvLoadOptions;
};

export type ActivePluginView = {
  pluginId: string;
  viewId: string;
  title: string;
  presentation: PluginViewPresentation;
  frameOpacity: number | null;
  html: string;
};
