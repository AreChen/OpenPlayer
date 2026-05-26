import { activeThemeFromAppearance, themeStyleVariables } from "../app/theme";
import type {
  AppearanceState,
  MediaItem,
  MpvSnapshot,
  SettingsSection,
  ShortcutAction,
} from "../app/types";
import type { AppLocale, AppStrings } from "../i18n";
import { useAppearanceSettings } from "./useAppearanceSettings";
import { usePlayerPreferences } from "./usePlayerPreferences";
import { useSettingsDialogState } from "./useSettingsDialogState";
import { useShellPreviewSettings } from "./useShellPreviewSettings";

type UsePlayerSettingsDomainOptions = {
  appearanceState: AppearanceState | null;
  setAppearanceState: (state: AppearanceState) => void;
  setPlayerPreferences: Parameters<typeof usePlayerPreferences>[0]["setPlayerPreferences"];
  media: MediaItem | null;
  isPickerOpen: boolean;
  setIsPickerOpen: (isPickerOpen: boolean) => void;
  locale: AppLocale;
  t: AppStrings;
  setContextMenu: (position: null) => void;
  setMediaPanelMode: (mode: null) => void;
  setRecordingShortcutAction: (action: ShortcutAction | null) => void;
  onError: (error: unknown) => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
};

export function usePlayerSettingsDomain({
  appearanceState,
  setAppearanceState,
  setPlayerPreferences,
  media,
  isPickerOpen,
  setIsPickerOpen,
  locale,
  t,
  setContextMenu,
  setMediaPanelMode,
  setRecordingShortcutAction,
  onError,
  applyCommandSnapshot,
}: UsePlayerSettingsDomainOptions) {
  const {
    isSettingsOpen,
    setIsSettingsOpen,
    settingsSection,
    setSettingsSection,
    settingsDialogRef,
    openSettingsDialog,
    closeSettingsDialog,
  } = useSettingsDialogState({
    onBeforeOpen: () => {
      setContextMenu(null);
      setMediaPanelMode(null);
    },
    onShortcutRecordingChange: setRecordingShortcutAction,
  });
  const activeTheme = activeThemeFromAppearance(appearanceState);
  const appearanceStyle = themeStyleVariables(appearanceState);
  const {
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
  } = useShellPreviewSettings(t.settings.shellPreview);
  const {
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
  } = useAppearanceSettings({
    appearanceState,
    setAppearanceState,
    isMediaLoaded: Boolean(media),
    isPickerOpen,
    setIsPickerOpen,
    locale,
    dialogText: {
      openPlayerPlugin: t.dialog.openPlayerPlugin,
      themePlugin: t.dialog.themePlugin,
    },
    onError,
    applyCommandSnapshot,
  });
  const { setIncognitoMode, setQuietKeyboardControls, setLanguageMode } = usePlayerPreferences({
    setPlayerPreferences,
    onError,
  });

  return {
    isSettingsOpen,
    setIsSettingsOpen,
    settingsSection: settingsSection as SettingsSection,
    setSettingsSection,
    settingsDialogRef,
    openSettingsDialog,
    closeSettingsDialog,
    activeTheme,
    appearanceStyle,
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
    setIncognitoMode,
    setQuietKeyboardControls,
    setLanguageMode,
  };
}
