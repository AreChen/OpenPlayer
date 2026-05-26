import { useEffect, useRef, useState } from "react";
import type { SettingsSection, ShortcutAction } from "../app/types";

type UseSettingsDialogStateOptions = {
  onBeforeOpen: () => void;
  onShortcutRecordingChange: (action: ShortcutAction | null) => void;
};

export function useSettingsDialogState({
  onBeforeOpen,
  onShortcutRecordingChange,
}: UseSettingsDialogStateOptions) {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [settingsSection, setSettingsSection] = useState<SettingsSection>("appearance");
  const settingsDialogRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (isSettingsOpen) {
      settingsDialogRef.current?.focus();
    } else {
      onShortcutRecordingChange(null);
    }
  }, [isSettingsOpen, onShortcutRecordingChange]);

  function openSettingsDialog() {
    onBeforeOpen();
    setSettingsSection("appearance");
    setIsSettingsOpen(true);
  }

  function closeSettingsDialog() {
    setIsSettingsOpen(false);
  }

  return {
    isSettingsOpen,
    setIsSettingsOpen,
    settingsSection,
    setSettingsSection,
    settingsDialogRef,
    openSettingsDialog,
    closeSettingsDialog,
  };
}
