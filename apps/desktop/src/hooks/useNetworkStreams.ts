import { useState, type FormEvent as ReactFormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { nextMediaItemId, normalizeNetworkStreamInput, streamNameFromUrl } from "../app/media";
import { platformUnsupportedPlaybackMessage } from "../app/playback";
import type { AppStrings } from "../i18n";
import type { MediaItem, MpvLoadOptions, NetworkStreamHistoryEntry, PlatformSupport, PlayerPreferences } from "../app/types";

type PreparedMediaOpen = {
  item: MediaItem;
  loadOptions: MpvLoadOptions;
};

type UseNetworkStreamsOptions = {
  platformSupport: PlatformSupport | null;
  t: AppStrings;
  playerPreferences: PlayerPreferences;
  preparePluginMediaOpen: (item: MediaItem, source: "stream", loadOptions?: MpvLoadOptions) => Promise<PreparedMediaOpen>;
  openMpvPath: (path: string, loadOptions?: MpvLoadOptions) => Promise<void>;
  setPlaybackError: (error: string | null) => void;
  setQueue: (queue: MediaItem[]) => void;
  setCurrentIndex: (index: number | null) => void;
  setIsPlaylistOpen: (isOpen: boolean) => void;
  closeFloatingPlaybackMenus: () => void;
  closeContextMenu: () => void;
};

export function useNetworkStreams({
  platformSupport,
  t,
  playerPreferences,
  preparePluginMediaOpen,
  openMpvPath,
  setPlaybackError,
  setQueue,
  setCurrentIndex,
  setIsPlaylistOpen,
  closeFloatingPlaybackMenus,
  closeContextMenu,
}: UseNetworkStreamsOptions) {
  const [networkStreamHistory, setNetworkStreamHistory] = useState<NetworkStreamHistoryEntry[]>([]);
  const [isNetworkStreamDialogOpen, setIsNetworkStreamDialogOpen] = useState(false);
  const [networkStreamUrl, setNetworkStreamUrl] = useState("");
  const [networkStreamError, setNetworkStreamError] = useState<string | null>(null);

  function openNetworkStreamDialog() {
    closeContextMenu();
    closeFloatingPlaybackMenus();
    setNetworkStreamError(null);
    setIsNetworkStreamDialogOpen(true);
  }

  function closeNetworkStreamDialog() {
    setIsNetworkStreamDialogOpen(false);
    setNetworkStreamError(null);
  }

  async function submitNetworkStream(event?: ReactFormEvent<HTMLFormElement>) {
    event?.preventDefault();
    const rawUrl = networkStreamUrl.trim();
    await openNetworkStreamFromInput(rawUrl);
  }

  async function openNetworkStreamFromInput(rawUrl: string, fallbackName: string | null = null) {
    try {
      const normalizedUrl = normalizeNetworkStreamInput(rawUrl);
      let name = streamNameFromUrl(normalizedUrl, fallbackName);
      if (!playerPreferences.incognitoMode) {
        const entries = await invoke<NetworkStreamHistoryEntry[]>("network_stream_history_remember", {
          entry: {
            url: normalizedUrl,
            name,
            updatedAt: Date.now(),
          },
        });
        if (Array.isArray(entries)) {
          setNetworkStreamHistory(entries);
          const activeEntry = entries.find((entry) => entry.url === normalizedUrl) ?? entries[0];
          if (activeEntry) {
            name = activeEntry.name;
          }
        }
      }
      closeNetworkStreamDialog();
      setNetworkStreamUrl("");
      await openRuntimeStream(normalizedUrl, name);
    } catch (error) {
      setNetworkStreamError(error instanceof Error ? error.message : String(error));
    }
  }

  function openNetworkStreamHistoryEntry(entry: NetworkStreamHistoryEntry) {
    setNetworkStreamUrl(entry.url);
    openNetworkStreamFromInput(entry.url, entry.name).catch((error: unknown) => {
      setNetworkStreamError(error instanceof Error ? error.message : String(error));
    });
  }

  function clearNetworkStreamHistory() {
    invoke<NetworkStreamHistoryEntry[]>("network_stream_history_clear")
      .then((entries) => setNetworkStreamHistory(Array.isArray(entries) ? entries : []))
      .catch((error: unknown) => {
        setNetworkStreamError(error instanceof Error ? error.message : String(error));
      });
  }

  async function openRuntimeStream(url: string, name: string | null = null, loadOptions: MpvLoadOptions = {}) {
    if (platformSupport && !platformSupport.mpvEmbedVideo) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return;
    }

    const item: MediaItem = {
      id: nextMediaItemId(),
      name: streamNameFromUrl(url, name),
      path: url,
    };
    const prepared = await preparePluginMediaOpen(item, "stream", loadOptions);
    setPlaybackError(null);
    setQueue([prepared.item]);
    setCurrentIndex(0);
    setIsPlaylistOpen(false);
    await openMpvPath(prepared.item.path, prepared.loadOptions);
  }

  return {
    networkStreamHistory,
    setNetworkStreamHistory,
    isNetworkStreamDialogOpen,
    networkStreamUrl,
    networkStreamError,
    setNetworkStreamUrl,
    setNetworkStreamError,
    openNetworkStreamDialog,
    closeNetworkStreamDialog,
    submitNetworkStream,
    openNetworkStreamHistoryEntry,
    clearNetworkStreamHistory,
    openRuntimeStream,
  };
}
