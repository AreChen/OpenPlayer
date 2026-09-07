import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { usePersistentStateSync } from "./usePersistentStateSync";
import { defaultShellPreviewExtensions } from "../app/media";
import type {
  AppVersionInfo,
  AppearanceState,
  NetworkStreamHistoryEntry,
  PlatformSupport,
  PlaybackHistoryEntry,
  PlaybackSettings,
  PlayerPreferences,
  ShellPreviewFormatInfo,
} from "../app/types";

type UseBackendStateSyncOptions = {
  onPlatformSupport: (support: PlatformSupport) => void;
  onPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void;
  onNetworkStreamHistory: (entries: NetworkStreamHistoryEntry[]) => void;
  onAppearanceState: (state: AppearanceState) => void;
  onPlayerPreferences: (preferences: PlayerPreferences) => void;
  onPlaybackSettings: (settings: PlaybackSettings) => void;
  onAppVersion: (version: AppVersionInfo) => void;
  onAlwaysOnTop: (enabled: boolean) => void;
  onShellPreviewFormats: (formats: ShellPreviewFormatInfo[], selectedExtensions: string[]) => void;
  onSystemFontFamilies: (fonts: string[]) => void;
  onStartupMediaPaths: (paths: string[]) => void;
};

export function useBackendStateSync({
  onPlatformSupport,
  onPlaybackHistory,
  onNetworkStreamHistory,
  onAppearanceState,
  onPlayerPreferences,
  onPlaybackSettings,
  onAppVersion,
  onAlwaysOnTop,
  onShellPreviewFormats,
  onSystemFontFamilies,
  onStartupMediaPaths,
}: UseBackendStateSyncOptions) {
  useEffect(() => {
    let disposed = false;
    invoke<PlatformSupport>("platform_support")
      .then((support) => {
        if (!disposed) {
          onPlatformSupport(support);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load platform support metadata", error);
      });

    invoke<AppVersionInfo>("app_version")
      .then((version) => {
        if (!disposed) {
          onAppVersion(version);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load app version metadata", error);
      });

    invoke<boolean>("window_always_on_top_state")
      .then((enabled) => {
        if (!disposed) {
          onAlwaysOnTop(Boolean(enabled));
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load always-on-top state", error);
      });

    invoke<ShellPreviewFormatInfo[]>("shell_preview_formats")
      .then((formats) => {
        if (!disposed && Array.isArray(formats)) {
          onShellPreviewFormats(formats, defaultShellPreviewExtensions(formats));
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load Explorer preview formats", error);
      });

    invoke<string[]>("system_font_families")
      .then((fonts) => {
        if (!disposed && Array.isArray(fonts)) {
          onSystemFontFamilies(fonts.filter((font) => typeof font === "string" && font.trim().length > 0));
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load system fonts", error);
      });

    invoke<string[]>("startup_media_paths")
      .then((paths) => {
        if (!disposed && Array.isArray(paths) && paths.length > 0) {
          onStartupMediaPaths(paths);
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load startup media paths", error);
      });

    return () => {
      disposed = true;
    };
  }, []);

  usePersistentStateSync({
    onAppearanceState,
    onPlayerPreferences,
    onPlaybackSettings,
    onPlaybackHistory,
    onNetworkStreamHistory,
  });
}
