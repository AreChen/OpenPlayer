import type { AppStrings } from "../i18n";
import { OPENPLAYER_SHORTCUTS_STORAGE_KEY, TEXT_ENTRY_INPUT_TYPES, defaultShortcutBindings, shortcutActions } from "./constants";
import type { ShortcutAction, ShortcutBindings, ShortcutDefinition } from "./types";

export function normalizeShortcutKey(key: string) {
  const aliases: Record<string, string> = {
    " ": "Space",
    Spacebar: "Space",
    Esc: "Escape",
    Left: "ArrowLeft",
    Right: "ArrowRight",
    Up: "ArrowUp",
    Down: "ArrowDown",
    Del: "Delete",
  };
  const normalized = aliases[key] ?? key;
  if (normalized.length === 1 && /[a-z]/i.test(normalized)) {
    return normalized.toUpperCase();
  }

  return normalized;
}

export function keyboardEventToChord(event: KeyboardEvent) {
  const key = normalizeShortcutKey(event.key);
  if (["Alt", "Control", "Meta", "Shift"].includes(key)) {
    return null;
  }

  const parts: string[] = [];
  if (event.ctrlKey) {
    parts.push("Ctrl");
  }
  if (event.metaKey) {
    parts.push("Meta");
  }
  if (event.altKey) {
    parts.push("Alt");
  }
  if (event.shiftKey) {
    parts.push("Shift");
  }

  parts.push(key);
  return parts.join("+");
}

export function formatShortcutChord(chord: string | null, t: AppStrings) {
  if (!chord) {
    return t.common.unset;
  }

  return chord
    .split("+")
    .map((part) => {
      const labels: Record<string, string> = {
        ArrowDown: "↓",
        ArrowLeft: "←",
        ArrowRight: "→",
        ArrowUp: "↑",
        Escape: "Esc",
        Meta: "Win",
        Space: "Space",
      };
      return labels[part] ?? part;
    })
    .join(" + ");
}

export function shortcutDefinitionsFor(t: AppStrings): ShortcutDefinition[] {
  return [
    { action: "openMedia", label: t.shortcuts.actions.openMedia, group: t.shortcuts.groups.file },
    { action: "togglePlayback", label: t.shortcuts.actions.togglePlayback, group: t.shortcuts.groups.playback },
    { action: "restart", label: t.shortcuts.actions.restart, group: t.shortcuts.groups.playback },
    { action: "togglePlaylist", label: t.shortcuts.actions.togglePlaylist, group: t.shortcuts.groups.playback },
    { action: "seekBackward", label: t.shortcuts.actions.seekBackward, group: t.shortcuts.groups.seek },
    { action: "seekForward", label: t.shortcuts.actions.seekForward, group: t.shortcuts.groups.seek },
    { action: "frameBackward", label: t.shortcuts.actions.frameBackward, group: t.shortcuts.groups.frame },
    { action: "frameForward", label: t.shortcuts.actions.frameForward, group: t.shortcuts.groups.frame },
    { action: "volumeDown", label: t.shortcuts.actions.volumeDown, group: t.shortcuts.groups.volume },
    { action: "volumeUp", label: t.shortcuts.actions.volumeUp, group: t.shortcuts.groups.volume },
    { action: "toggleFullscreen", label: t.shortcuts.actions.toggleFullscreen, group: t.shortcuts.groups.window },
    { action: "toggleAlwaysOnTop", label: t.shortcuts.actions.toggleAlwaysOnTop, group: t.shortcuts.groups.window },
    { action: "openSettings", label: t.shortcuts.actions.openSettings, group: t.shortcuts.groups.window },
  ];
}

export function readShortcutBindings() {
  try {
    const stored = window.localStorage.getItem(OPENPLAYER_SHORTCUTS_STORAGE_KEY);
    if (!stored) {
      return defaultShortcutBindings;
    }

    const parsed = JSON.parse(stored) as Partial<Record<ShortcutAction, unknown>>;
    const merged: ShortcutBindings = { ...defaultShortcutBindings };
    for (const action of shortcutActions) {
      const value = parsed[action];
      if (typeof value === "string" || value === null) {
        merged[action] = value;
      }
    }

    return merged;
  } catch {
    return defaultShortcutBindings;
  }
}

export function isShortcutAction(value: unknown): value is ShortcutAction {
  return typeof value === "string" && shortcutActions.includes(value as ShortcutAction);
}

export function isTextEntryShortcutTarget(target: EventTarget | null) {
  if (!(target instanceof Element)) {
    return false;
  }

  const editable = target.closest("textarea, [contenteditable='true'], [role='textbox']");
  if (editable) {
    return true;
  }

  const input = target.closest("input");
  return input instanceof HTMLInputElement && TEXT_ENTRY_INPUT_TYPES.has(input.type);
}

export function isPointerInsideSelector(target: EventTarget | null, selector: string) {
  return target instanceof Element && Boolean(target.closest(selector));
}

export function isPointerInsidePlaybackControl(target: EventTarget | null) {
  return isPointerInsideSelector(
    target,
    [
      "[contenteditable='true']",
      ".context-menu",
      ".drop-overlay",
      ".media-panel",
      ".playback-error",
      ".playlist-drawer",
      ".resize-region",
      ".settings-dialog",
      ".transport",
      ".window-controls",
    ].join(", "),
  );
}

export function isWheelInsideInteractiveSurface(target: EventTarget | null) {
  return isPointerInsideSelector(
    target,
    [
      "button",
      "input",
      "select",
      "textarea",
      "[contenteditable='true']",
      ".context-menu",
      ".media-panel",
      ".playlist-drawer",
      ".settings-dialog",
      ".transport",
      ".window-controls",
    ].join(", "),
  );
}

export function releaseShortcutFocusTarget(target: EventTarget | null) {
  if (isTextEntryShortcutTarget(target)) {
    return;
  }

  if (document.activeElement instanceof HTMLElement) {
    document.activeElement.blur();
  }
}
