import { useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { HISTORY_WRITE_INTERVAL_MS } from "../app/constants";
import { mediaNameFromPath } from "../app/media";
import type { PlaybackHistoryEntry, PlayerPreferences } from "../app/types";

type UsePlaybackHistoryProgressOptions = {
  playerPreferences: PlayerPreferences;
  setPlaybackHistory: (entries: PlaybackHistoryEntry[]) => void;
};

export function usePlaybackHistoryProgress({ playerPreferences, setPlaybackHistory }: UsePlaybackHistoryProgressOptions) {
  const lastHistoryWriteRef = useRef(0);

  function rememberPlaybackProgress(path: string, position: number, snapshotDuration: number, force = false) {
    if (!path || playerPreferences.incognitoMode) {
      return;
    }

    const now = Date.now();
    if (!force && now - lastHistoryWriteRef.current < HISTORY_WRITE_INTERVAL_MS) {
      return;
    }

    lastHistoryWriteRef.current = now;
    invoke<PlaybackHistoryEntry[]>("history_remember", {
      entry: {
        path,
        name: mediaNameFromPath(path),
        position: Number.isFinite(position) ? Math.max(0, position) : 0,
        duration: Number.isFinite(snapshotDuration) ? Math.max(0, snapshotDuration) : 0,
        updatedAt: now,
      },
    })
      .then((entries) => setPlaybackHistory(Array.isArray(entries) ? entries : []))
      .catch((error: unknown) => {
        console.warn("Failed to remember playback progress", error);
      });
  }

  return {
    rememberPlaybackProgress,
  };
}
