import {
  AboutSettingsPanel,
  AppearanceSettingsPanel,
  PlaybackSettingsPanel,
  PluginSettingsPanel,
  ShortcutSettingsPanel,
} from "./SettingsPanels";
import type { SettingsDialogContentProps } from "./SettingsDialog.types";

export function SettingsDialogContent({
  t,
  locale,
  settingsSection,
  appearanceState,
  activeTheme,
  playerPreferences,
  playbackHistoryLength,
  plugins,
  expandedPluginIds,
  isPickerOpen,
  systemFontFamilies,
  shellPreviewFormats,
  selectedShellPreviewFormats,
  shellPreviewRegistrationStatus,
  isRegisteringShellPreview,
  shortcutDefinitions,
  shortcutBindings,
  recordingShortcutAction,
  appVersion,
  updateState,
  onResetAppearance,
  onSelectTheme,
  onSetAccentOverride,
  onSetLanguageMode,
  onImportPluginPackage,
  onImportPluginDirectory,
  onImportThemePlugin,
  onOpenExternalUrl,
  onSetPluginEnabled,
  onTogglePluginSettingsExpanded,
  onUninstallPlugin,
  onSetPluginSettingValue,
  onChoosePluginDirectory,
  onOpenPluginDirectory,
  onClearPlaybackHistory,
  onSetIncognitoMode,
  onSetQuietKeyboardControls,
  onToggleAllShellPreviewFormats,
  onResetShellPreviewFormatsToDefault,
  onOpenDefaultAppsSettings,
  onRegisterShellPreviews,
  onToggleShellPreviewFormat,
  onResetShortcutBindings,
  onStartRecordingShortcut,
  onAssignShortcut,
  onCheckForUpdates,
  onOpenUpdateDownload,
}: SettingsDialogContentProps) {
  if (settingsSection === "appearance") {
    return (
      <AppearanceSettingsPanel
        t={t}
        locale={locale}
        appearanceState={appearanceState}
        activeTheme={activeTheme}
        playerPreferences={playerPreferences}
        onResetAppearance={onResetAppearance}
        onSelectTheme={onSelectTheme}
        onSetAccentOverride={onSetAccentOverride}
        onSetLanguageMode={onSetLanguageMode}
      />
    );
  }

  if (settingsSection === "plugins") {
    return (
      <PluginSettingsPanel
        t={t}
        locale={locale}
        plugins={plugins}
        expandedPluginIds={expandedPluginIds}
        isPickerOpen={isPickerOpen}
        systemFontFamilies={systemFontFamilies}
        onImportPluginPackage={onImportPluginPackage}
        onImportPluginDirectory={onImportPluginDirectory}
        onImportThemePlugin={onImportThemePlugin}
        onOpenExternalUrl={onOpenExternalUrl}
        onSetPluginEnabled={onSetPluginEnabled}
        onTogglePluginSettingsExpanded={onTogglePluginSettingsExpanded}
        onUninstallPlugin={onUninstallPlugin}
        onSetPluginSettingValue={onSetPluginSettingValue}
        onChoosePluginDirectory={onChoosePluginDirectory}
        onOpenPluginDirectory={onOpenPluginDirectory}
      />
    );
  }

  if (settingsSection === "playback") {
    return (
      <PlaybackSettingsPanel
        t={t}
        playerPreferences={playerPreferences}
        playbackHistoryLength={playbackHistoryLength}
        shellPreviewFormats={shellPreviewFormats}
        selectedShellPreviewFormats={selectedShellPreviewFormats}
        shellPreviewRegistrationStatus={shellPreviewRegistrationStatus}
        isRegisteringShellPreview={isRegisteringShellPreview}
        onClearPlaybackHistory={onClearPlaybackHistory}
        onSetIncognitoMode={onSetIncognitoMode}
        onSetQuietKeyboardControls={onSetQuietKeyboardControls}
        onToggleAllShellPreviewFormats={onToggleAllShellPreviewFormats}
        onResetShellPreviewFormatsToDefault={onResetShellPreviewFormatsToDefault}
        onOpenDefaultAppsSettings={onOpenDefaultAppsSettings}
        onRegisterShellPreviews={onRegisterShellPreviews}
        onToggleShellPreviewFormat={onToggleShellPreviewFormat}
      />
    );
  }

  if (settingsSection === "shortcuts") {
    return (
      <ShortcutSettingsPanel
        t={t}
        shortcutDefinitions={shortcutDefinitions}
        shortcutBindings={shortcutBindings}
        recordingShortcutAction={recordingShortcutAction}
        onResetShortcutBindings={onResetShortcutBindings}
        onStartRecordingShortcut={onStartRecordingShortcut}
        onAssignShortcut={onAssignShortcut}
      />
    );
  }

  return (
    <AboutSettingsPanel
      t={t}
      appVersion={appVersion}
      updateState={updateState}
      onCheckForUpdates={onCheckForUpdates}
      onOpenUpdateDownload={onOpenUpdateDownload}
      onOpenExternalUrl={onOpenExternalUrl}
    />
  );
}
