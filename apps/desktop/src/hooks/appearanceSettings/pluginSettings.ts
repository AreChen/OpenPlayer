import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { localizedPluginText, normalizePluginSettingValue } from "../../app/pluginRuntime";
import type {
  AppearanceState,
  MpvSnapshot,
  PluginSettingDefinition,
  PluginSettingValue,
} from "../../app/types";
import { focusOverlayWindow } from "../../app/windowControls";

type UsePluginSettingActionsOptions = {
  appearanceState: AppearanceState | null;
  isMediaLoaded: boolean;
  isPickerOpen: boolean;
  setIsPickerOpen: (isPickerOpen: boolean) => void;
  locale: string;
  updateAppearance: (request: Promise<AppearanceState>) => Promise<void>;
  onError: (error: unknown) => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
};

export function usePluginSettingActions({
  appearanceState,
  isMediaLoaded,
  isPickerOpen,
  setIsPickerOpen,
  locale,
  updateAppearance,
  onError,
  applyCommandSnapshot,
}: UsePluginSettingActionsOptions) {
  function setPluginSettingValue(
    pluginId: string,
    setting: PluginSettingDefinition,
    value: PluginSettingValue,
  ) {
    const nextValue = normalizePluginSettingValue(setting, value);
    if (nextValue === null) {
      return;
    }

    void updateAppearance(
      invoke<AppearanceState>("appearance_set_plugin_setting", {
        pluginId,
        settingId: setting.id,
        value: nextValue,
      }).then(async (state) => {
        if (isMediaLoaded && setting.mpvProperty) {
          await applyPluginMpvSetting(setting, nextValue);
        }
        return state;
      }),
    );
  }

  async function choosePluginDirectory(pluginId: string, setting: PluginSettingDefinition) {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        directory: true,
        multiple: false,
        title: localizedPluginText(setting.label, setting.labelI18n, locale),
      });
      if (typeof selection === "string") {
        setPluginSettingValue(pluginId, setting, selection);
      }
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  function openPluginDirectory(setting: PluginSettingDefinition) {
    const value = typeof setting.value === "string" ? setting.value.trim() : "";
    if (!value) {
      return;
    }
    invoke("window_open_directory", { path: value }).catch(onError).finally(focusOverlayWindow);
  }

  async function applyPluginMpvSetting(
    setting: PluginSettingDefinition,
    value: PluginSettingValue,
  ) {
    if (!setting.mpvProperty) {
      return;
    }
    try {
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_set_plugin_property", {
        property: setting.mpvProperty,
        value,
      });
      applyCommandSnapshot(snapshot);
    } catch (error) {
      onError(error);
    }
  }

  async function applyStoredPluginMpvSettings(snapshot: MpvSnapshot) {
    let activeSnapshot = snapshot;
    const pluginSettings = (appearanceState?.plugins ?? [])
      .filter((plugin) => plugin.enabled)
      .flatMap((plugin) => plugin.settings)
      .filter((setting) => setting.mpvProperty);
    for (const setting of pluginSettings) {
      activeSnapshot = await invoke<MpvSnapshot>("mpv_embed_set_plugin_property", {
        property: setting.mpvProperty,
        value: normalizePluginSettingValue(setting, setting.value) ?? setting.defaultValue,
      });
    }
    return activeSnapshot;
  }

  return {
    setPluginSettingValue,
    choosePluginDirectory,
    openPluginDirectory,
    applyStoredPluginMpvSettings,
  };
}
