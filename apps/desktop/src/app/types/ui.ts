export type SettingsSection = "appearance" | "plugins" | "playback" | "shortcuts" | "about";
export type MediaPanelMode = "speed" | "tracks" | "loop";

export type PendingWindowDrag = {
  pointerId: number;
  startX: number;
  startY: number;
};

export type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";

export type ManualResizeDrag = {
  pointerId: number;
  direction: ResizeDirection;
  lastX: number;
  lastY: number;
  pendingDeltaX: number;
  pendingDeltaY: number;
  animationFrameId: number | null;
  resizeCommandInFlight: boolean;
  finishing: boolean;
};

export type ResizeFeedback = {
  direction: ResizeDirection;
  active: boolean;
};

export type ShortcutAction =
  | "openMedia"
  | "togglePlayback"
  | "restart"
  | "togglePlaylist"
  | "seekBackward"
  | "seekForward"
  | "frameForward"
  | "frameBackward"
  | "volumeDown"
  | "volumeUp"
  | "toggleFullscreen"
  | "toggleAlwaysOnTop"
  | "openSettings";

export type ShortcutBindings = Record<ShortcutAction, string | null>;

export type ShortcutDefinition = {
  action: ShortcutAction;
  label: string;
  group: string;
};

export type ContextMenuPosition = {
  x: number;
  y: number;
};

export type IconName =
  | "camera"
  | "close"
  | "cpu"
  | "folder"
  | "folderAdd"
  | "fullscreen"
  | "info"
  | "list"
  | "maximize"
  | "minimize"
  | "next"
  | "palette"
  | "pause"
  | "pin"
  | "play"
  | "plugin"
  | "preview"
  | "previous"
  | "record"
  | "restart"
  | "settings"
  | "stop"
  | "stream"
  | "tracks"
  | "tv"
  | "volume"
  | "volumeMuted";
