import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_PLAYBACK_SETTINGS } from "../app/constants";
import { mergePlaybackSettings, normalizePlaybackSettings } from "../app/playback";
import type {
  HardwareDecodingMode,
  LoopMode,
  PlaybackSettings,
  PlaybackSettingsUpdate,
  TimeDisplayMode,
} from "../app/types";
import type { MutableRefObject } from "react";

type UsePlaybackSettingsStoreOptions = {
  previousAudibleVolumeRef: MutableRefObject<number>;
  hardwareDecodingModeRef: MutableRefObject<HardwareDecodingMode>;
  setVolumeLevel: (volume: number) => void;
  setPlaybackSpeedValue: (speed: number) => void;
  setHardwareDecodingModeValue: (mode: HardwareDecodingMode) => void;
  setIsVideoFillEnabled: (enabled: boolean) => void;
  setTimeDisplayModeValue: (mode: TimeDisplayMode) => void;
  setLoopModeValue: (mode: LoopMode) => void;
};

export function usePlaybackSettingsStore({
  previousAudibleVolumeRef,
  hardwareDecodingModeRef,
  setVolumeLevel,
  setPlaybackSpeedValue,
  setHardwareDecodingModeValue,
  setIsVideoFillEnabled,
  setTimeDisplayModeValue,
  setLoopModeValue,
}: UsePlaybackSettingsStoreOptions) {
  const [playbackSettings, setPlaybackSettings] = useState<PlaybackSettings>(DEFAULT_PLAYBACK_SETTINGS);
  const playbackSettingsRef = useRef<PlaybackSettings>(DEFAULT_PLAYBACK_SETTINGS);

  function applyPlaybackSettingsFromStore(settings: Partial<PlaybackSettings> | null | undefined) {
    const normalized = normalizePlaybackSettings(settings);
    playbackSettingsRef.current = normalized;
    setPlaybackSettings(normalized);
    setVolumeLevel(normalized.volume / 100);
    if (normalized.volume > 0) {
      previousAudibleVolumeRef.current = normalized.volume / 100;
    }
    setPlaybackSpeedValue(normalized.playbackSpeed);
    setHardwareDecodingModeValue(normalized.hwdecMode);
    hardwareDecodingModeRef.current = normalized.hwdecMode;
    setIsVideoFillEnabled(normalized.videoFill);
    setTimeDisplayModeValue(normalized.timeDisplayMode);
    setLoopModeValue(normalized.loopMode);
    return normalized;
  }

  function persistPlaybackSettings(update: PlaybackSettingsUpdate) {
    const optimistic = mergePlaybackSettings(playbackSettingsRef.current, update);
    playbackSettingsRef.current = optimistic;
    setPlaybackSettings(optimistic);
    invoke<PlaybackSettings>("playback_settings_update", { settings: update })
      .then(applyPlaybackSettingsFromStore)
      .catch((error: unknown) => {
        console.warn("Failed to persist playback settings", error);
      });
  }

  async function loadPlaybackSettings() {
    try {
      const settings = await invoke<PlaybackSettings>("playback_settings_state");
      return applyPlaybackSettingsFromStore(settings);
    } catch (error) {
      console.warn("Failed to resolve playback settings", error);
      return playbackSettingsRef.current;
    }
  }

  return {
    playbackSettings,
    applyPlaybackSettingsFromStore,
    persistPlaybackSettings,
    loadPlaybackSettings,
  };
}
