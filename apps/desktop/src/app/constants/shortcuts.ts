import type { ShortcutAction, ShortcutBindings } from "../types";

export const OPENPLAYER_SHORTCUTS_STORAGE_KEY = "openplayer.shortcuts.v3";

export const TEXT_ENTRY_INPUT_TYPES = new Set([
  "",
  "date",
  "datetime-local",
  "email",
  "month",
  "number",
  "password",
  "search",
  "tel",
  "text",
  "time",
  "url",
  "week",
]);

export const shortcutActions: ShortcutAction[] = [
  "openMedia",
  "togglePlayback",
  "restart",
  "togglePlaylist",
  "seekBackward",
  "seekForward",
  "frameBackward",
  "frameForward",
  "volumeDown",
  "volumeUp",
  "toggleFullscreen",
  "toggleAlwaysOnTop",
  "openSettings",
];

export const defaultShortcutBindings: ShortcutBindings = {
  openMedia: "Ctrl+O",
  togglePlayback: "Space",
  restart: "R",
  togglePlaylist: "P",
  seekBackward: "ArrowLeft",
  seekForward: "ArrowRight",
  frameBackward: "D",
  frameForward: "F",
  volumeDown: "ArrowDown",
  volumeUp: "ArrowUp",
  toggleFullscreen: "Enter",
  toggleAlwaysOnTop: "\\",
  openSettings: "Ctrl+,",
};
