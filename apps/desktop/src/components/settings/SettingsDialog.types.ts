import type { RefObject } from "react";
import type {
  AppVersionInfo,
  AppearanceState,
  PlayerPreferences,
  PluginRuntimeLogEntry,
  PluginSettingDefinition,
  PluginSettingValue,
  SettingsSection,
  ShellPreviewFormatInfo,
  ShortcutAction,
  ShortcutBindings,
  ShortcutDefinition,
  ThemeCatalogItem,
  ThemePluginSummary,
  UpdateState,
} from "../../app/types";
import type { AppLocale, AppStrings, LanguageMode } from "../../i18n";

export type SettingsDialogProps = {
  t: AppStrings;
  locale: AppLocale;
  dialogRef: RefObject<HTMLElement | null>;
  settingsSection: SettingsSection;
  appearanceState: AppearanceState | null;
  activeTheme: ThemeCatalogItem | null;
  playerPreferences: PlayerPreferences;
  playbackHistoryLength: number;
  plugins: ThemePluginSummary[];
  pluginRuntimeLogs: PluginRuntimeLogEntry[];
  expandedPluginIds: Set<string>;
  isPickerOpen: boolean;
  systemFontFamilies: string[];
  shellPreviewFormats: ShellPreviewFormatInfo[];
  selectedShellPreviewFormats: string[];
  shellPreviewRegistrationStatus: string | null;
  isRegisteringShellPreview: boolean;
  shortcutDefinitions: ShortcutDefinition[];
  shortcutBindings: ShortcutBindings;
  recordingShortcutAction: ShortcutAction | null;
  appVersion: AppVersionInfo | null;
  updateState: UpdateState;
  onSectionChange: (section: SettingsSection) => void;
  onClose: () => void;
  onResetAppearance: () => void;
  onSelectTheme: (themeId: string) => void;
  onSetAccentOverride: (accent: string | null) => void;
  onSetLanguageMode: (mode: LanguageMode) => void;
  onImportPluginPackage: () => void;
  onImportPluginDirectory: () => void;
  onImportThemePlugin: () => void;
  onOpenExternalUrl: (url: string | null | undefined) => void;
  onSetPluginEnabled: (pluginId: string, enabled: boolean) => void;
  onTogglePluginSettingsExpanded: (pluginId: string) => void;
  onUninstallPlugin: (pluginId: string) => void;
  onSetPluginSettingValue: (
    pluginId: string,
    setting: PluginSettingDefinition,
    value: PluginSettingValue,
  ) => void;
  onChoosePluginDirectory: (pluginId: string, setting: PluginSettingDefinition) => void;
  onOpenPluginDirectory: (setting: PluginSettingDefinition) => void;
  onClearPlaybackHistory: () => void;
  onSetIncognitoMode: (enabled: boolean) => void;
  onSetQuietKeyboardControls: (enabled: boolean) => void;
  onToggleAllShellPreviewFormats: () => void;
  onResetShellPreviewFormatsToDefault: () => void;
  onOpenDefaultAppsSettings: () => void;
  onRegisterShellPreviews: () => void;
  onToggleShellPreviewFormat: (extension: string) => void;
  onResetShortcutBindings: () => void;
  onStartRecordingShortcut: (action: ShortcutAction | null) => void;
  onAssignShortcut: (action: ShortcutAction, shortcut: string | null) => void;
  onCheckForUpdates: () => void;
  onOpenUpdateDownload: () => void;
};

export type SettingsDialogContentProps = Omit<
  SettingsDialogProps,
  "dialogRef" | "onClose" | "onSectionChange"
>;
