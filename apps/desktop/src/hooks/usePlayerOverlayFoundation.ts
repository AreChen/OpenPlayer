import { useMemo, type MutableRefObject } from "react";
import { loopModeOptionsFor } from "../app/playback";
import { pluginShortcutDefinitionsFor, shortcutDefinitionsFor } from "../app/shortcuts";
import { browserLanguages } from "../app/theme";
import type {
  HardwareDecodingMode,
  AppearanceState,
  LoopMode,
  PlatformSupport,
  PlayerPreferences,
  TimeDisplayMode,
} from "../app/types";
import { resolveLocale, translations } from "../i18n";
import { useContextMenuState } from "./useContextMenuState";
import { usePlaybackErrorReporter } from "./usePlaybackErrorReporter";
import { usePlaybackPanelState } from "./usePlaybackPanelState";
import { usePlaybackSettingsStore } from "./usePlaybackSettingsStore";
import { usePlayerFeedback } from "./usePlayerFeedback";
import { useShortcutSettings } from "./useShortcutSettings";

type UsePlayerOverlayFoundationOptions = {
  appearanceState: AppearanceState | null;
  playerPreferences: PlayerPreferences;
  platformSupport: PlatformSupport | null;
  previousAudibleVolumeRef: MutableRefObject<number>;
  hardwareDecodingModeRef: MutableRefObject<HardwareDecodingMode>;
  setVolumeLevel: (volume: number) => void;
  setPlaybackSpeedValue: (speed: number) => void;
  setHardwareDecodingModeValue: (mode: HardwareDecodingMode) => void;
  setIsVideoFillEnabled: (enabled: boolean) => void;
  setTimeDisplayModeValue: (mode: TimeDisplayMode) => void;
  setLoopModeValue: (mode: LoopMode) => void;
};

export function usePlayerOverlayFoundation({
  appearanceState,
  playerPreferences,
  platformSupport,
  previousAudibleVolumeRef,
  hardwareDecodingModeRef,
  setVolumeLevel,
  setPlaybackSpeedValue,
  setHardwareDecodingModeValue,
  setIsVideoFillEnabled,
  setTimeDisplayModeValue,
  setLoopModeValue,
}: UsePlayerOverlayFoundationOptions) {
  const locale = resolveLocale(playerPreferences.languageMode, browserLanguages());
  const t = translations[locale];
  const loopModeOptions = loopModeOptionsFor(t);
  const shortcutDefinitions = useMemo(() => [
    ...shortcutDefinitionsFor(t),
    ...pluginShortcutDefinitionsFor(appearanceState?.plugins ?? [], locale),
  ], [appearanceState?.plugins, locale, t]);
  const shortcutSettings = useShortcutSettings();
  const playbackPanels = usePlaybackPanelState();
  const contextMenuState = useContextMenuState();
  const feedback = usePlayerFeedback();
  const { reportPlaybackError } = usePlaybackErrorReporter({
    platformSupport,
    t,
    setPlaybackError: feedback.setPlaybackError,
  });
  const {
    applyPlaybackSettingsFromStore,
    persistPlaybackSettings,
    loadPlaybackSettings,
  } = usePlaybackSettingsStore({
    previousAudibleVolumeRef,
    hardwareDecodingModeRef,
    setVolumeLevel,
    setPlaybackSpeedValue,
    setHardwareDecodingModeValue,
    setIsVideoFillEnabled,
    setTimeDisplayModeValue,
    setLoopModeValue,
  });

  return {
    locale,
    t,
    loopModeOptions,
    shortcutDefinitions,
    ...shortcutSettings,
    ...playbackPanels,
    ...contextMenuState,
    ...feedback,
    reportPlaybackError,
    applyPlaybackSettingsFromStore,
    persistPlaybackSettings,
    loadPlaybackSettings,
  };
}
