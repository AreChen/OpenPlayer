import { useEffect, useState } from "react";
import { OPENPLAYER_SHORTCUTS_STORAGE_KEY, defaultShortcutBindings } from "../app/constants";
import { assignShortcutBinding, readShortcutBindings } from "../app/shortcuts";
import type { ShortcutAction, ShortcutBindings } from "../app/types";

export function useShortcutSettings() {
  const [shortcutBindings, setShortcutBindings] = useState<ShortcutBindings>(readShortcutBindings);
  const [recordingShortcutAction, setRecordingShortcutAction] = useState<ShortcutAction | null>(null);

  function assignShortcut(action: ShortcutAction, chord: string | null) {
    setShortcutBindings((bindings) => assignShortcutBinding(bindings, action, chord));
  }

  function resetShortcutBindings() {
    setShortcutBindings(defaultShortcutBindings);
    setRecordingShortcutAction(null);
  }

  useEffect(() => {
    try {
      window.localStorage.setItem(OPENPLAYER_SHORTCUTS_STORAGE_KEY, JSON.stringify(shortcutBindings));
    } catch (error) {
      console.warn("Failed to persist shortcut settings", error);
    }
  }, [shortcutBindings]);

  return {
    shortcutBindings,
    recordingShortcutAction,
    setRecordingShortcutAction,
    assignShortcut,
    resetShortcutBindings,
  };
}
