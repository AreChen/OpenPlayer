import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppearanceState, MpvSnapshot } from "../app/types";
import { focusOverlayWindow } from "../app/windowControls";
import {
  usePluginImportActions,
  type AppearanceDialogText,
} from "./appearanceSettings/pluginImports";
import { usePluginSettingActions } from "./appearanceSettings/pluginSettings";

type UseAppearanceSettingsOptions = {
  appearanceState: AppearanceState | null;
  setAppearanceState: (state: AppearanceState) => void;
  isMediaLoaded: boolean;
  isPickerOpen: boolean;
  setIsPickerOpen: (isPickerOpen: boolean) => void;
  locale: string;
  dialogText: AppearanceDialogText;
  onError: (error: unknown) => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
};

export function useAppearanceSettings({
  appearanceState,
  setAppearanceState,
  isMediaLoaded,
  isPickerOpen,
  setIsPickerOpen,
  locale,
  dialogText,
  onError,
  applyCommandSnapshot,
}: UseAppearanceSettingsOptions) {
  const [expandedPluginIds, setExpandedPluginIds] = useState<Set<string>>(() => new Set());

  async function updateAppearance(request: Promise<AppearanceState>) {
    try {
      setAppearanceState(await request);
    } catch (error) {
      onError(error);
    } finally {
      focusOverlayWindow();
    }
  }

  function selectTheme(themeId: string) {
    void updateAppearance(invoke<AppearanceState>("appearance_set_theme", { themeId }));
  }

  function setAccentOverride(accent: string | null) {
    void updateAppearance(invoke<AppearanceState>("appearance_set_accent_override", { accent }));
  }

  function resetAppearance() {
    void updateAppearance(invoke<AppearanceState>("appearance_reset"));
  }

  function setThemePluginEnabled(pluginId: string, enabled: boolean) {
    void updateAppearance(invoke<AppearanceState>("appearance_set_plugin_enabled", { pluginId, enabled }));
  }

  function uninstallPlugin(pluginId: string) {
    void updateAppearance(invoke<AppearanceState>("appearance_uninstall_plugin", { pluginId }));
  }

  function togglePluginSettingsExpanded(pluginId: string) {
    setExpandedPluginIds((current) => {
      const next = new Set(current);
      if (next.has(pluginId)) {
        next.delete(pluginId);
      } else {
        next.add(pluginId);
      }
      return next;
    });
  }

  const {
    setPluginSettingValue,
    choosePluginDirectory,
    openPluginDirectory,
    applyStoredPluginMpvSettings,
  } = usePluginSettingActions({
    appearanceState,
    isMediaLoaded,
    isPickerOpen,
    setIsPickerOpen,
    locale,
    updateAppearance,
    onError,
    applyCommandSnapshot,
  });
  const { importPluginPackage, importPluginDirectory, importThemePlugin } =
    usePluginImportActions({
      isPickerOpen,
      setIsPickerOpen,
      dialogText,
      updateAppearance,
      onError,
    });

  return {
    expandedPluginIds,
    updateAppearance,
    selectTheme,
    setAccentOverride,
    resetAppearance,
    setThemePluginEnabled,
    uninstallPlugin,
    togglePluginSettingsExpanded,
    setPluginSettingValue,
    choosePluginDirectory,
    openPluginDirectory,
    applyStoredPluginMpvSettings,
    importPluginPackage,
    importPluginDirectory,
    importThemePlugin,
  };
}
