import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  isShortcutAction,
  isTextEntryShortcutTarget,
  keyboardEventToChord,
  releaseShortcutFocusTarget,
} from "../app/shortcuts";
import type { ContextMenuPosition, ShortcutAction, ShortcutBindings, ShortcutDefinition } from "../app/types";

type UseShortcutBridgeOptions = {
  contextMenu: ContextMenuPosition | null;
  isSettingsOpen: boolean;
  isNetworkStreamDialogOpen: boolean;
  recordingShortcutAction: ShortcutAction | null;
  shortcutBindings: ShortcutBindings;
  shortcutDefinitions: ShortcutDefinition[];
  onRecordUserActivity: () => void;
  onRecordShortcutActivity: (action: ShortcutAction) => void;
  onPerformShortcutAction: (action: ShortcutAction) => void;
  onAssignShortcut: (action: ShortcutAction, chord: string | null) => void;
  onCancelRecordingShortcut: () => void;
  onCloseContextMenu: () => void;
  onCloseSettings: () => void;
  onCloseNetworkStreamDialog: () => void;
};

export function useShortcutBridge({
  contextMenu,
  isSettingsOpen,
  isNetworkStreamDialogOpen,
  recordingShortcutAction,
  shortcutBindings,
  shortcutDefinitions,
  onRecordUserActivity,
  onRecordShortcutActivity,
  onPerformShortcutAction,
  onAssignShortcut,
  onCancelRecordingShortcut,
  onCloseContextMenu,
  onCloseSettings,
  onCloseNetworkStreamDialog,
}: UseShortcutBridgeOptions) {
  const shortcutKeyDownRef = useRef<(event: KeyboardEvent) => void>(() => undefined);
  const nativeShortcutActionRef = useRef<(action: ShortcutAction) => void>(() => undefined);

  shortcutKeyDownRef.current = (event: KeyboardEvent) => {
    if (recordingShortcutAction) {
      onRecordUserActivity();
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        onCancelRecordingShortcut();
        return;
      }

      if (event.key === "Backspace" || event.key === "Delete") {
        onAssignShortcut(recordingShortcutAction, null);
        onCancelRecordingShortcut();
        return;
      }

      const chord = keyboardEventToChord(event);
      if (chord) {
        onAssignShortcut(recordingShortcutAction, chord);
        onCancelRecordingShortcut();
      }
      return;
    }

    if (event.key === "Escape") {
      onRecordUserActivity();
      if (contextMenu) {
        event.preventDefault();
        onCloseContextMenu();
        return;
      }

      if (isSettingsOpen) {
        event.preventDefault();
        onCloseSettings();
        return;
      }

      if (isNetworkStreamDialogOpen) {
        event.preventDefault();
        onCloseNetworkStreamDialog();
      }
      return;
    }

    if (contextMenu || isSettingsOpen || isNetworkStreamDialogOpen || isTextEntryShortcutTarget(event.target)) {
      return;
    }

    const chord = keyboardEventToChord(event);
    const shortcut = shortcutDefinitions.find((definition) => shortcutBindings[definition.action] === chord);
    if (!shortcut) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    releaseShortcutFocusTarget(event.target);
    onRecordShortcutActivity(shortcut.action);
    onPerformShortcutAction(shortcut.action);
  };

  nativeShortcutActionRef.current = (action: ShortcutAction) => {
    if (contextMenu || isSettingsOpen || isNetworkStreamDialogOpen || recordingShortcutAction) {
      return;
    }

    onRecordShortcutActivity(action);
    onPerformShortcutAction(action);
  };

  useEffect(() => {
    function handleGlobalKeyDown(event: KeyboardEvent) {
      shortcutKeyDownRef.current(event);
    }

    window.addEventListener("keydown", handleGlobalKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", handleGlobalKeyDown, { capture: true });
  }, []);

  useEffect(() => {
    let unlistenShortcut: (() => void) | null = null;
    let disposed = false;

    listen<string>("openplayer-native-shortcut", (event) => {
      if (isShortcutAction(event.payload)) {
        nativeShortcutActionRef.current(event.payload);
      }
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenShortcut = unlisten;
        }
      })
      .catch((error: unknown) => {
        console.warn("Native shortcut listener failed", error);
      });

    return () => {
      disposed = true;
      unlistenShortcut?.();
    };
  }, []);

  useEffect(() => {
    invoke("window_update_shortcuts", { bindings: shortcutBindings }).catch((error: unknown) => {
      console.warn("Native shortcut update failed", error);
    });
  }, [shortcutBindings]);

  useEffect(() => {
    const enabled = !contextMenu && !isSettingsOpen && !isNetworkStreamDialogOpen && !recordingShortcutAction;
    invoke("window_set_shortcuts_enabled", { enabled }).catch((error: unknown) => {
      console.warn("Native shortcut state update failed", error);
    });
  }, [contextMenu, isSettingsOpen, isNetworkStreamDialogOpen, recordingShortcutAction]);
}
