import { invoke } from "@tauri-apps/api/core";
import { focusOverlayWindow, runWindowCommand } from "../app/windowControls";
import type { MediaItem } from "../app/types";

type UseWindowActionsOptions = {
  media: MediaItem | null;
  setIsAlwaysOnTop: (enabled: boolean) => void;
  showAlwaysOnTopFeedback: (enabled: boolean) => void;
  onError: (error: unknown) => void;
};

export function useWindowActions({ media, setIsAlwaysOnTop, showAlwaysOnTopFeedback, onError }: UseWindowActionsOptions) {
  async function openExternalUrl(url: string | null | undefined) {
    if (!url) {
      return;
    }

    try {
      await invoke("app_open_url", { url });
    } catch (error) {
      onError(error);
    } finally {
      focusOverlayWindow();
    }
  }

  function toggleFullscreen() {
    runWindowCommand("window_toggle_fullscreen");
  }

  function toggleAlwaysOnTop() {
    invoke<boolean>("window_toggle_always_on_top")
      .then((enabled) => {
        setIsAlwaysOnTop(enabled);
        showAlwaysOnTopFeedback(enabled);
        focusOverlayWindow();
      })
      .catch(onError);
  }

  function openCurrentFileLocation() {
    if (!media) {
      return;
    }

    invoke("window_reveal_path", { path: media.path })
      .then(focusOverlayWindow)
      .catch(onError);
  }

  return {
    openExternalUrl,
    toggleFullscreen,
    toggleAlwaysOnTop,
    openCurrentFileLocation,
  };
}
