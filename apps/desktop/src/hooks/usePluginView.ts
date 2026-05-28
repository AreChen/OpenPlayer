import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { buildPluginViewDocument, localizedPluginText, resolvePluginViewFrameOpacity } from "../app/pluginRuntime";
import type { AppLocale } from "../i18n";
import type { ActivePluginView, AppearanceState, ContextMenuPosition, MediaPanelMode, PluginViewHtml, ThemeCatalogItem } from "../app/types";

type UsePluginViewOptions = {
  appearanceState: AppearanceState | null;
  activeTheme: ThemeCatalogItem | null;
  locale: AppLocale;
  setContextMenu: (contextMenu: ContextMenuPosition | null) => void;
  setIsPlaylistOpen: (isOpen: boolean) => void;
  setMediaPanelMode: (mode: MediaPanelMode | null) => void;
  closeNetworkStreamDialog: () => void;
  setIsSettingsOpen: (isOpen: boolean) => void;
};

export function usePluginView({
  appearanceState,
  activeTheme,
  locale,
  setContextMenu,
  setIsPlaylistOpen,
  setMediaPanelMode,
  closeNetworkStreamDialog,
  setIsSettingsOpen,
}: UsePluginViewOptions) {
  const [activePluginView, setActivePluginView] = useState<ActivePluginView | null>(null);
  const pluginViewFrameRef = useRef<HTMLIFrameElement | null>(null);

  async function openPluginView(pluginId: string, viewId: string) {
    const plugin = appearanceState?.plugins.find((candidate) => candidate.id === pluginId && candidate.enabled);
    if (!plugin) {
      throw new Error(`plugin is unavailable: ${pluginId}`);
    }
    const view = plugin.views.find((candidate) => candidate.id === viewId);
    if (!view) {
      throw new Error(`plugin view is unavailable: ${pluginId}.${viewId}`);
    }
    const viewHtml = await invoke<PluginViewHtml>("appearance_plugin_view_html", { pluginId, viewId });
    setContextMenu(null);
    setIsPlaylistOpen(false);
    setMediaPanelMode(null);
    closeNetworkStreamDialog();
    setIsSettingsOpen(false);
    setActivePluginView({
      pluginId: viewHtml.pluginId,
      viewId: viewHtml.viewId,
      title: localizedPluginText(view.title, view.titleI18n, locale),
      presentation: view.presentation,
      frameOpacity: resolvePluginViewFrameOpacity(plugin, view),
      html: viewHtml.html,
    });
  }

  function closePluginView() {
    invoke("mpv_wall_close").catch((error: unknown) => {
      console.warn("Failed to close plugin native wall", error);
    });
    setActivePluginView(null);
  }

  const activePluginViewPlugin = activePluginView ? appearanceState?.plugins.find((plugin) => plugin.id === activePluginView.pluginId) ?? null : null;
  const activePluginViewDefinition =
    activePluginView && activePluginViewPlugin
      ? activePluginViewPlugin.views.find((view) => view.id === activePluginView.viewId) ?? null
      : null;
  const resolvedActivePluginView =
    activePluginView && activePluginViewPlugin && activePluginViewDefinition
      ? {
          ...activePluginView,
          frameOpacity: resolvePluginViewFrameOpacity(activePluginViewPlugin, activePluginViewDefinition),
        }
      : activePluginView;
  const activePluginViewThemeTokens = activeTheme
    ? {
        ...activeTheme.tokens,
        accent: appearanceState?.accentOverride ?? activeTheme.tokens.accent,
      }
    : null;
  const activePluginViewDocument =
    activePluginView && activePluginViewPlugin && activePluginViewThemeTokens
      ? buildPluginViewDocument(activePluginView.html, activePluginViewPlugin, locale, activePluginViewThemeTokens)
      : null;

  return {
    activePluginView: resolvedActivePluginView,
    activePluginViewDocument,
    pluginViewFrameRef,
    openPluginView,
    closePluginView,
  };
}
