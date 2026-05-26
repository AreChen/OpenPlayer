import { invoke } from "@tauri-apps/api/core";
import { mediaItemFromHistory } from "../../app/media";
import type {
  MediaItem,
  MpvLoadOptions,
  PlaybackHistoryEntry,
  PluginMediaOpenInput,
} from "../../app/types";

type PreparedMediaOpen = {
  item: MediaItem;
  loadOptions: MpvLoadOptions;
};

type OpenHistoryEntryOptions = {
  entry: PlaybackHistoryEntry;
  setQueue: (queue: MediaItem[]) => void;
  setCurrentIndex: (index: number | null) => void;
  setIsPlaylistOpen: (isOpen: boolean) => void;
  preparePluginMediaOpen: (
    item: MediaItem,
    source: PluginMediaOpenInput["source"],
    loadOptions?: MpvLoadOptions,
  ) => Promise<PreparedMediaOpen>;
  openMpvPath: (path: string, loadOptions?: MpvLoadOptions) => Promise<void>;
  onError: (error: unknown) => void;
};

export function openHistoryEntryForQueue({
  entry,
  setQueue,
  setCurrentIndex,
  setIsPlaylistOpen,
  preparePluginMediaOpen,
  openMpvPath,
  onError,
}: OpenHistoryEntryOptions) {
  const item = mediaItemFromHistory(entry);
  setQueue([item]);
  setCurrentIndex(0);
  setIsPlaylistOpen(false);
  preparePluginMediaOpen(item, "history")
    .then(async (prepared) => {
      setQueue([prepared.item]);
      await openMpvPath(prepared.item.path, prepared.loadOptions);
    })
    .catch(onError);
}

export function clearPlaybackHistoryForQueue(
  setPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void,
  onError: (error: unknown) => void,
) {
  invoke<PlaybackHistoryEntry[]>("history_clear")
    .then((entries) => setPlaybackHistory(Array.isArray(entries) ? entries : []))
    .catch(onError);
}
