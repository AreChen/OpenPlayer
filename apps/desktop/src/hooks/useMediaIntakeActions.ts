import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { playableExtensions } from "../app/constants";
import { isOpenPlayerPluginPackagePath } from "../app/media";
import { platformUnsupportedPlaybackMessage } from "../app/playback";
import { focusOverlayWindow } from "../app/windowControls";
import type { AppStrings } from "../i18n";
import type { AppearanceState, PlatformSupport } from "../app/types";

type UseMediaIntakeActionsOptions = {
  platformSupport: PlatformSupport | null;
  t: AppStrings;
  isPickerOpen: boolean;
  setIsPickerOpen: (isOpen: boolean) => void;
  setPlaybackError: (error: string | null) => void;
  updateAppearance: (request: Promise<AppearanceState>) => Promise<void>;
  replaceQueueWithMediaPaths: (paths: string[]) => Promise<void>;
  appendMediaPaths: (paths: string[]) => Promise<void>;
  onError: (error: unknown) => void;
};

export function useMediaIntakeActions({
  platformSupport,
  t,
  isPickerOpen,
  setIsPickerOpen,
  setPlaybackError,
  updateAppearance,
  replaceQueueWithMediaPaths,
  appendMediaPaths,
  onError,
}: UseMediaIntakeActionsOptions) {
  function showUnsupportedPlayback() {
    if (platformSupport && !platformSupport.mpvEmbedVideo) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return true;
    }
    return false;
  }

  async function openNativeMediaFiles() {
    if (isPickerOpen) {
      return;
    }

    if (showUnsupportedPlayback()) {
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: t.dialog.mediaFiles, extensions: playableExtensions }],
      });
      const paths = typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
      await replaceQueueWithMediaPaths(paths);
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function appendNativeMediaFiles() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: t.dialog.mediaFiles, extensions: playableExtensions }],
      });
      const paths = typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
      await appendMediaPaths(paths);
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function appendNativeMediaFolder() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        directory: true,
        multiple: false,
      });
      const folderPath = typeof selection === "string" ? selection : null;
      if (!folderPath) {
        return;
      }

      const paths = await invoke<string[]>("media_files_in_directory", { path: folderPath });
      await appendMediaPaths(paths);
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  async function playDroppedPaths(paths: string[]) {
    const pluginPackagePaths = paths.filter(isOpenPlayerPluginPackagePath);
    if (pluginPackagePaths.length > 0) {
      setPlaybackError(null);
      for (const pluginPath of pluginPackagePaths) {
        await updateAppearance(invoke<AppearanceState>("appearance_import_plugin_package", { path: pluginPath }));
      }
    }

    const mediaCandidatePaths = paths.filter((path) => !isOpenPlayerPluginPackagePath(path));
    if (!mediaCandidatePaths.length) {
      focusOverlayWindow();
      return;
    }

    if (showUnsupportedPlayback()) {
      return;
    }

    setPlaybackError(null);
    const mediaPaths = await invoke<string[]>("media_files_from_paths", { paths: mediaCandidatePaths });
    await replaceQueueWithMediaPaths(mediaPaths);
    focusOverlayWindow();
  }

  return {
    openNativeMediaFiles,
    appendNativeMediaFiles,
    appendNativeMediaFolder,
    playDroppedPaths,
  };
}
