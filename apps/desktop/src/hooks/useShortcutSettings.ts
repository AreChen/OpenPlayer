import { useEffect, useState } from "react";
import { OPENPLAYER_SHORTCUTS_STORAGE_KEY, defaultShortcutBindings } from "../app/constants";
import { readShortcutBindings } from "../app/shortcuts";
import type { ShortcutAction, ShortcutBindings, ShortcutDefinition } from "../app/types";

export function useShortcutSettings(shortcutDefinitions: ShortcutDefinition[]) {
  const [shortcutBindings, setShortcutBindings] = useState<ShortcutBindings>(readShortcutBindings);
  const [recordingShortcutAction, setRecordingShortcutAction] = useState<ShortcutAction | null>(null);

  function assignShortcut(action: ShortcutAction, chord: string | null) {
    setShortcutBindings((bindings) => {
      const next = { ...bindings, [action]: chord };
      if (chord) {
        for (const definition of shortcutDefinitions) {
          if (definition.action !== action && next[definition.action] === chord) {
            next[definition.action] = null;
          }
        }
      }

      return next;
    });
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
