import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { subtitleExtensions } from "../app/constants";
import { clampSubtitleDelay } from "../app/playback";
import { focusOverlayWindow } from "../app/windowControls";
import type { AppStrings } from "../i18n";
import type { MediaItem, MediaPlaybackSettings, MpvSnapshot, SelectableTrackKind } from "../app/types";

type UseTrackActionsOptions = {
  media: MediaItem | null;
  isPickerOpen: boolean;
  t: AppStrings;
  setIsPickerOpen: (isOpen: boolean) => void;
  setSubtitleDelayValue: (delay: number) => void;
  invalidatePendingSnapshots: () => void;
  applyCommandSnapshot: (snapshot: MpvSnapshot) => void;
  onError: (error: unknown) => void;
};

export function useTrackActions({
  media,
  isPickerOpen,
  t,
  setIsPickerOpen,
  setSubtitleDelayValue,
  invalidatePendingSnapshots,
  applyCommandSnapshot,
  onError,
}: UseTrackActionsOptions) {
  function persistMediaPlaybackSettings(path: string, settings: { subtitleTrackId?: number | null }) {
    if (!path) {
      return;
    }

    invoke<MediaPlaybackSettings>("playback_media_settings_update", { path, settings }).catch((error: unknown) => {
      console.warn("Failed to persist media playback settings", error);
    });
  }

  function setSubtitleDelay(delay: number) {
    if (!media) {
      return;
    }

    const nextDelay = clampSubtitleDelay(delay);
    setSubtitleDelayValue(nextDelay);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_subtitle_delay", { delay: nextDelay })
      .then(applyCommandSnapshot)
      .catch(onError);
  }

  function selectTrack(kind: SelectableTrackKind, trackId: number | null) {
    if (!media) {
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_select_track", { kind, trackId })
      .then((snapshot) => {
        applyCommandSnapshot(snapshot);
        if (kind === "subtitle" && media) {
          persistMediaPlaybackSettings(media.path, { subtitleTrackId: trackId });
        }
      })
      .catch(onError);
  }

  async function addExternalSubtitle() {
    if (!media || isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    try {
      const selection = await open({
        multiple: false,
        filters: [{ name: t.dialog.subtitle, extensions: subtitleExtensions }],
      });
      if (typeof selection !== "string") {
        return;
      }

      invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_add_subtitle", { path: selection });
      applyCommandSnapshot(snapshot);
      const selectedSubtitle = snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
      persistMediaPlaybackSettings(media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
    } catch (error) {
      onError(error);
    } finally {
      setIsPickerOpen(false);
      focusOverlayWindow();
    }
  }

  return {
    persistMediaPlaybackSettings,
    setSubtitleDelay,
    selectTrack,
    addExternalSubtitle,
  };
}
