import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { STORE_SYNC_INTERVAL_MS } from "../app/constants";
import type {
  AppearanceState,
  AppearanceSyncState,
  PlayerPreferences,
  PlaybackSettings,
  PlaybackSyncState,
  PlaybackHistoryEntry,
  NetworkStreamHistoryEntry,
} from "../app/types";

type Callbacks = {
  onAppearanceState: (state: AppearanceState) => void;
  onPlayerPreferences: (state: PlayerPreferences) => void;
  onPlaybackSettings: (state: PlaybackSettings) => void;
  onPlaybackHistory: (state: PlaybackHistoryEntry[]) => void;
  onNetworkStreamHistory: (state: NetworkStreamHistoryEntry[]) => void;
};

export function usePersistentStateSync(callbacks: Callbacks) {
  const callbacksRef = useRef(callbacks);
  callbacksRef.current = callbacks;
  useEffect(() => {
    let disposed = false;
    let inFlight = false;
    const signatures = new Map<keyof Callbacks, string>();
    function apply<K extends keyof Callbacks>(key: K, value: Parameters<Callbacks[K]>[0]) {
      if (disposed) return;
      const signature = JSON.stringify(value);
      // Playback settings can have optimistic local edits; always reconcile their authoritative value.
      if (key !== "onPlaybackSettings" && signatures.get(key) === signature) return;
      (callbacksRef.current[key] as (next: Parameters<Callbacks[K]>[0]) => void)(value);
      signatures.set(key, signature);
    }
    async function sync() {
      if (disposed || inFlight) return;
      inFlight = true;
      await Promise.allSettled([
        invoke<AppearanceSyncState>("appearance_sync_state").then((state) => {
          apply("onAppearanceState", state.appearance);
          apply("onPlayerPreferences", state.preferences);
        }).catch((error: unknown) => console.warn("Failed to sync appearance state", error)),
        invoke<PlaybackSyncState>("playback_sync_state").then((state) => {
          apply("onPlaybackSettings", state.settings);
          apply("onPlaybackHistory", state.history);
          apply("onNetworkStreamHistory", state.networkStreams);
        }).catch((error: unknown) => console.warn("Failed to sync playback state", error)),
      ]);
      inFlight = false;
    }
    void sync();
    const timer = window.setInterval(() => void sync(), STORE_SYNC_INTERVAL_MS);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, []);
}
