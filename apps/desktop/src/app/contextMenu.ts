import type { ContextMenuEntry } from "../components/ContextMenu";
import type { AppLocale, AppStrings } from "../i18n";
import { localizedPluginText } from "./pluginRuntime";
import type { PluginActionInstance, ShortcutBindings } from "./types";

type ContextMenuItemsOptions = {
  t: AppStrings;
  locale: AppLocale;
  shortcutBindings: ShortcutBindings;
  isPickerOpen: boolean;
  isMediaLoaded: boolean;
  isPlaying: boolean;
  isAlwaysOnTop: boolean;
  pluginContextMenuActions: PluginActionInstance[];
  isPluginActionDisabled: (action: PluginActionInstance["action"]) => boolean;
  onExecutePluginAction: (action: PluginActionInstance) => void;
  onOpenNativeMediaFiles: () => void;
  onAppendNativeMediaFiles: () => void;
  onAppendNativeMediaFolder: () => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onRestartPlayback: () => void;
  onOpenCurrentFileLocation: () => void;
  onToggleFullscreen: () => void;
  onToggleAlwaysOnTop: () => void;
  onOpenSettingsDialog: () => void;
  onCloseWindow: () => void;
};

export function buildContextMenuItems({
  t,
  locale,
  shortcutBindings,
  isPickerOpen,
  isMediaLoaded,
  isPlaying,
  isAlwaysOnTop,
  pluginContextMenuActions,
  isPluginActionDisabled,
  onExecutePluginAction,
  onOpenNativeMediaFiles,
  onAppendNativeMediaFiles,
  onAppendNativeMediaFolder,
  onTogglePlayback,
  onStopPlayback,
  onRestartPlayback,
  onOpenCurrentFileLocation,
  onToggleFullscreen,
  onToggleAlwaysOnTop,
  onOpenSettingsDialog,
  onCloseWindow,
}: ContextMenuItemsOptions): ContextMenuEntry[] {
  return [
    { type: "item", id: "open", label: t.contextMenu.openMedia, icon: "folder", shortcut: shortcutBindings.openMedia, disabled: isPickerOpen, onSelect: onOpenNativeMediaFiles },
    { type: "item", id: "append-files", label: t.contextMenu.appendFiles, icon: "folderAdd", disabled: isPickerOpen, onSelect: onAppendNativeMediaFiles },
    { type: "item", id: "append-folder", label: t.contextMenu.appendFolder, icon: "folderAdd", disabled: isPickerOpen, onSelect: onAppendNativeMediaFolder },
    {
      type: "item",
      id: "play",
      label: isPlaying ? t.contextMenu.pause : isMediaLoaded ? t.contextMenu.play : t.contextMenu.openMedia,
      icon: isPlaying ? "pause" : "play",
      shortcut: shortcutBindings.togglePlayback,
      disabled: !isMediaLoaded && isPickerOpen,
      onSelect: onTogglePlayback,
    },
    { type: "item", id: "stop", label: t.contextMenu.stop, icon: "stop", disabled: !isMediaLoaded, onSelect: onStopPlayback },
    { type: "item", id: "restart", label: t.contextMenu.restart, icon: "restart", shortcut: shortcutBindings.restart, disabled: !isMediaLoaded, onSelect: onRestartPlayback },
    { type: "separator", id: "playback-separator" },
    { type: "item", id: "open-location", label: t.contextMenu.openFileLocation, icon: "folder", disabled: !isMediaLoaded, onSelect: onOpenCurrentFileLocation },
    ...(pluginContextMenuActions.length > 0 ? [{ type: "separator" as const, id: "plugin-actions-separator" }] : []),
    ...pluginContextMenuActions.map((instance) => ({
      type: "item" as const,
      id: `plugin:${instance.plugin.id}:${instance.action.id}`,
      label: localizedPluginText(instance.action.label, instance.action.labelI18n, locale),
      icon: instance.action.icon ?? "plugin",
      disabled: isPluginActionDisabled(instance.action),
      onSelect: () => onExecutePluginAction(instance),
    })),
    { type: "item", id: "fullscreen", label: t.contextMenu.fullscreen, icon: "fullscreen", shortcut: shortcutBindings.toggleFullscreen, onSelect: onToggleFullscreen },
    { type: "item", id: "always-on-top", label: isAlwaysOnTop ? t.contextMenu.disableAlwaysOnTop : t.contextMenu.alwaysOnTop, icon: "pin", shortcut: shortcutBindings.toggleAlwaysOnTop, onSelect: onToggleAlwaysOnTop },
    { type: "item", id: "settings", label: t.contextMenu.settings, icon: "settings", shortcut: shortcutBindings.openSettings, onSelect: onOpenSettingsDialog },
    { type: "separator", id: "window-separator" },
    { type: "item", id: "close", label: t.contextMenu.closeWindow, icon: "close", onSelect: onCloseWindow },
  ];
}
