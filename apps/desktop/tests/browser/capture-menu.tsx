import React from "react";
import { createRoot } from "react-dom/client";
import { ContextMenu } from "../../src/components/ContextMenu";
import { buildContextMenuItems } from "../../src/app/contextMenu";
import { translations } from "../../src/i18n";
import "../../src/styles.css";

const locale = new URLSearchParams(location.search).get("locale") === "zh-CN" ? "zh-CN" : "en-US";
const t = translations[locale];
const noop = () => {};
const items = buildContextMenuItems({
  t, locale, shortcutBindings: {} as never, isPickerOpen: false, isMediaLoaded: true,
  isPlaying: true, isAlwaysOnTop: false, pluginContextMenuActions: [],
  isPluginActionDisabled: () => false, onExecutePluginAction: noop,
  onOpenNativeMediaFiles: noop, onAppendNativeMediaFiles: noop, onAppendNativeMediaFolder: noop,
  onTogglePlayback: noop, onStopPlayback: noop, onRestartPlayback: noop,
  onOpenCurrentFileLocation: noop, onToggleFullscreen: noop, onToggleAlwaysOnTop: noop,
  onOpenSettingsDialog: noop, onCloseWindow: noop,
  onEnterCaptureMode: () => { document.documentElement.dataset.capture = "entered"; },
});
createRoot(document.getElementById("root")!).render(<ContextMenu
  t={t} items={items} onClose={noop}
  position={{ x: Math.max(8, innerWidth - 244), y: Math.max(8, innerHeight - 428) }}
/>);
