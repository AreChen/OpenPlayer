import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { defaultShellPreviewExtensions } from "../app/media";
import type { ShellPreviewFormatInfo, ShellPreviewRegistrationSummary } from "../app/types";
import { focusOverlayWindow } from "../app/windowControls";

type ShellPreviewText = {
  noSelection: string;
  registered: (count: number) => string;
  failed: (message: string) => string;
  openDefaultAppsFailed: (message: string) => string;
};

export function useShellPreviewSettings(t: ShellPreviewText) {
  const [shellPreviewFormats, setShellPreviewFormats] = useState<ShellPreviewFormatInfo[]>([]);
  const [selectedShellPreviewFormats, setSelectedShellPreviewFormats] = useState<string[]>([]);
  const [shellPreviewRegistrationStatus, setShellPreviewRegistrationStatus] = useState<string | null>(null);
  const [isRegisteringShellPreview, setIsRegisteringShellPreview] = useState(false);

  function loadShellPreviewFormats(formats: ShellPreviewFormatInfo[], selectedExtensions: string[]) {
    setShellPreviewFormats(formats);
    setSelectedShellPreviewFormats(selectedExtensions);
  }

  function toggleShellPreviewFormat(extension: string) {
    setShellPreviewRegistrationStatus(null);
    setSelectedShellPreviewFormats((selected) => {
      if (selected.includes(extension)) {
        return selected.filter((item) => item !== extension);
      }

      return [...selected, extension];
    });
  }

  function toggleAllShellPreviewFormats() {
    setShellPreviewRegistrationStatus(null);
    setSelectedShellPreviewFormats((selected) => {
      if (shellPreviewFormats.length > 0 && selected.length === shellPreviewFormats.length) {
        return [];
      }

      return shellPreviewFormats.map((format) => format.extension);
    });
  }

  function resetShellPreviewFormatsToDefault() {
    setShellPreviewRegistrationStatus(null);
    setSelectedShellPreviewFormats(defaultShellPreviewExtensions(shellPreviewFormats));
  }

  async function registerShellPreviews() {
    if (isRegisteringShellPreview) {
      return;
    }

    if (!selectedShellPreviewFormats.length) {
      setShellPreviewRegistrationStatus(t.noSelection);
      return;
    }

    setIsRegisteringShellPreview(true);
    setShellPreviewRegistrationStatus(null);
    try {
      const summary = await invoke<ShellPreviewRegistrationSummary>("shell_preview_register_formats", { selectedExtensions: selectedShellPreviewFormats });
      setShellPreviewRegistrationStatus(t.registered(summary.registeredCount));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setShellPreviewRegistrationStatus(t.failed(message));
    } finally {
      setIsRegisteringShellPreview(false);
      focusOverlayWindow();
    }
  }

  async function openDefaultAppsSettings() {
    try {
      await invoke("shell_preview_open_default_apps_settings");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setShellPreviewRegistrationStatus(t.openDefaultAppsFailed(message));
    }
  }

  return {
    shellPreviewFormats,
    selectedShellPreviewFormats,
    shellPreviewRegistrationStatus,
    isRegisteringShellPreview,
    loadShellPreviewFormats,
    toggleShellPreviewFormat,
    toggleAllShellPreviewFormats,
    resetShellPreviewFormatsToDefault,
    registerShellPreviews,
    openDefaultAppsSettings,
  };
}
