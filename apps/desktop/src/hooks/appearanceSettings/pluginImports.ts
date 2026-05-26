import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { pluginPackageExtensions, themePluginExtensions } from "../../app/constants";
import type { AppearanceState } from "../../app/types";
import { focusOverlayWindow } from "../../app/windowControls";

type AppearanceDialogText = {
  openPlayerPlugin: string;
  themePlugin: string;
};

type UsePluginImportActionsOptions = {
  isPickerOpen: boolean;
  setIsPickerOpen: (isPickerOpen: boolean) => void;
  dialogText: AppearanceDialogText;
  updateAppearance: (request: Promise<AppearanceState>) => Promise<void>;
  onError: (error: unknown) => void;
};

export function usePluginImportActions({
  isPickerOpen,
  setIsPickerOpen,
  dialogText,
  updateAppearance,
  onError,
}: UsePluginImportActionsOptions) {
  async function importPluginPackage() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        multiple: false,
        filters: [{ name: dialogText.openPlayerPlugin, extensions: pluginPackageExtensions }],
      });
      if (typeof selection !== "string") {
        return;
      }

      await updateAppearance(invoke<AppearanceState>("appearance_import_plugin_package", { path: selection }));
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function importPluginDirectory() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        directory: true,
        multiple: false,
      });
      if (typeof selection !== "string") {
        return;
      }

      await updateAppearance(invoke<AppearanceState>("appearance_import_plugin_directory", { path: selection }));
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function importThemePlugin() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        multiple: false,
        filters: [{ name: dialogText.themePlugin, extensions: themePluginExtensions }],
      });
      if (typeof selection !== "string") {
        return;
      }

      await updateAppearance(invoke<AppearanceState>("appearance_import_plugin_manifest", { path: selection }));
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  return {
    importPluginPackage,
    importPluginDirectory,
    importThemePlugin,
  };
}

export type { AppearanceDialogText };
