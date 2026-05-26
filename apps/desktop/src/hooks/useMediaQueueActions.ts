import { mediaItemFromPath, uniqueMediaPaths } from "../app/media";
import { platformUnsupportedPlaybackMessage } from "../app/playback";
import type { AppStrings } from "../i18n";
import type {
  LoopMode,
  MediaItem,
  MpvLoadOptions,
  PlatformSupport,
  PlaybackHistoryEntry,
  PluginMediaOpenInput,
} from "../app/types";
import type { MutableRefObject } from "react";
import {
  clearPlaybackHistoryForQueue,
  openHistoryEntryForQueue,
} from "./mediaQueueActions/history";
import {
  nextQueueIndexFor,
  previousQueueIndexFor,
} from "./mediaQueueActions/navigation";

type PreparedMediaOpen = {
  item: MediaItem;
  loadOptions: MpvLoadOptions;
};

type UseMediaQueueActionsOptions = {
  platformSupport: PlatformSupport | null;
  t: AppStrings;
  queue: MediaItem[];
  media: MediaItem | null;
  currentIndex: number | null;
  loopMode: LoopMode;
  handledEndedPathRef: MutableRefObject<string | null>;
  setQueue: (queue: MediaItem[] | ((current: MediaItem[]) => MediaItem[])) => void;
  setCurrentIndex: (index: number | null) => void;
  setIsPlaylistOpen: (isOpen: boolean) => void;
  setPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void;
  setPlaybackError: (error: string | null) => void;
  preparePluginMediaOpen: (item: MediaItem, source: PluginMediaOpenInput["source"], loadOptions?: MpvLoadOptions) => Promise<PreparedMediaOpen>;
  openMpvPath: (path: string, loadOptions?: MpvLoadOptions) => Promise<void>;
  restartPlayback: (autoplay?: boolean) => void;
  onError: (error: unknown) => void;
};

export function useMediaQueueActions({
  platformSupport,
  t,
  queue,
  media,
  currentIndex,
  loopMode,
  handledEndedPathRef,
  setQueue,
  setCurrentIndex,
  setIsPlaylistOpen,
  setPlaybackHistory,
  setPlaybackError,
  preparePluginMediaOpen,
  openMpvPath,
  restartPlayback,
  onError,
}: UseMediaQueueActionsOptions) {
  function showUnsupportedPlayback() {
    if (platformSupport && !platformSupport.mpvEmbedVideo) {
      setPlaybackError(platformUnsupportedPlaybackMessage(platformSupport, t));
      return true;
    }
    return false;
  }

  async function replaceQueueWithMediaPaths(paths: string[]) {
    if (showUnsupportedPlayback()) {
      return;
    }

    const nextQueue = uniqueMediaPaths(paths).map(mediaItemFromPath);
    if (!nextQueue.length) {
      return;
    }

    setQueue(nextQueue);
    setCurrentIndex(0);
    setIsPlaylistOpen(nextQueue.length > 1);
    const prepared = await preparePluginMediaOpen(nextQueue[0], "file");
    setQueue((current) => current.map((item, index) => (index === 0 ? prepared.item : item)));
    await openMpvPath(prepared.item.path, prepared.loadOptions);
  }

  async function appendMediaPaths(paths: string[]) {
    if (showUnsupportedPlayback()) {
      return;
    }

    const baseQueue = queue.length ? queue : media ? [media] : [];
    const appendedPaths = uniqueMediaPaths(paths, new Set(baseQueue.map((item) => item.path)));
    if (!appendedPaths.length) {
      return;
    }

    const nextQueue = [...baseQueue, ...appendedPaths.map(mediaItemFromPath)];
    const shouldStartPlayback = !media;
    setQueue(nextQueue);
    setCurrentIndex(shouldStartPlayback ? 0 : currentIndex ?? 0);
    setIsPlaylistOpen(nextQueue.length > 1);
    if (shouldStartPlayback) {
      const prepared = await preparePluginMediaOpen(nextQueue[0], "file");
      setQueue((current) => current.map((item, index) => (index === 0 ? prepared.item : item)));
      await openMpvPath(prepared.item.path, prepared.loadOptions);
    }
  }

  async function openQueueIndex(index: number) {
    const item = queue[index];
    if (!item) {
      return;
    }

    handledEndedPathRef.current = null;
    setCurrentIndex(index);
    const prepared = await preparePluginMediaOpen(item, "playlist");
    setQueue((current) => current.map((candidate, candidateIndex) => (candidateIndex === index ? prepared.item : candidate)));
    await openMpvPath(prepared.item.path, prepared.loadOptions);
  }

  function playQueueIndex(index: number) {
    openQueueIndex(index).catch(onError);
  }

  function chooseQueueItem(index: number) {
    if (index === currentIndex) {
      return;
    }

    playQueueIndex(index);
  }

  function previousQueueIndex() {
    return previousQueueIndexFor(currentIndex, queue.length, loopMode);
  }

  function nextQueueIndex() {
    return nextQueueIndexFor(currentIndex, queue.length, loopMode);
  }

  function playPreviousQueueItem() {
    const index = previousQueueIndex();
    if (index !== null) {
      playQueueIndex(index);
    }
  }

  function playNextQueueItem() {
    const index = nextQueueIndex();
    if (index !== null) {
      playQueueIndex(index);
    }
  }

  function openHistoryEntry(entry: PlaybackHistoryEntry) {
    openHistoryEntryForQueue({
      entry,
      setQueue,
      setCurrentIndex,
      setIsPlaylistOpen,
      preparePluginMediaOpen,
      openMpvPath,
      onError,
    });
  }

  function clearPlaybackHistory() {
    clearPlaybackHistoryForQueue(setPlaybackHistory, onError);
  }

  function handlePlaybackEnd(path: string) {
    if (!media || media.path !== path || handledEndedPathRef.current === path) {
      return;
    }

    if (loopMode === "off") {
      return;
    }

    handledEndedPathRef.current = path;
    if (loopMode === "one") {
      restartPlayback(true);
      return;
    }

    const index = nextQueueIndex();
    if (index !== null) {
      playQueueIndex(index);
    } else {
      restartPlayback(true);
    }
  }

  return {
    replaceQueueWithMediaPaths,
    appendMediaPaths,
    openQueueIndex,
    chooseQueueItem,
    previousQueueIndex,
    nextQueueIndex,
    playPreviousQueueItem,
    playNextQueueItem,
    openHistoryEntry,
    clearPlaybackHistory,
    handlePlaybackEnd,
  };
}
