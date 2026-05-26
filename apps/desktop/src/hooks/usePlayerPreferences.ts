import { invoke } from "@tauri-apps/api/core";
import type { PlayerPreferences } from "../app/types";
import type { LanguageMode } from "../i18n";
import { focusOverlayWindow } from "../app/windowControls";

type UsePlayerPreferencesOptions = {
  setPlayerPreferences: (preferences: PlayerPreferences) => void;
  onError: (error: unknown) => void;
};

export function usePlayerPreferences({ setPlayerPreferences, onError }: UsePlayerPreferencesOptions) {
  async function updatePlayerPreferences(request: Promise<PlayerPreferences>) {
    try {
      setPlayerPreferences(await request);
    } catch (error) {
      onError(error);
    } finally {
      focusOverlayWindow();
    }
  }

  function setIncognitoMode(enabled: boolean) {
    void updatePlayerPreferences(invoke<PlayerPreferences>("preferences_set_incognito_mode", { enabled }));
  }

  function setQuietKeyboardControls(enabled: boolean) {
    void updatePlayerPreferences(invoke<PlayerPreferences>("preferences_set_quiet_keyboard_controls", { enabled }));
  }

  function setLanguageMode(mode: LanguageMode) {
    void updatePlayerPreferences(invoke<PlayerPreferences>("preferences_set_language_mode", { mode }));
  }

  return {
    setIncognitoMode,
    setQuietKeyboardControls,
    setLanguageMode,
  };
}
