import type { MpvRecordingState, PlaybackSettings, PlayerPreferences, UpdateState } from "../types";

export const INACTIVE_RECORDING_STATE: MpvRecordingState = { active: false, path: null, format: null };

export const DEFAULT_PLAYER_PREFERENCES: PlayerPreferences = {
  incognitoMode: false,
  quietKeyboardControls: false,
  languageMode: "system",
};

export const DEFAULT_PLAYBACK_SETTINGS: PlaybackSettings = {
  volume: 82,
  loopMode: "off",
  hwdecMode: "hardware",
  playbackSpeed: 1,
  videoFill: false,
  timeDisplayMode: "timecode",
};

export const DEFAULT_UPDATE_STATE: UpdateState = {
  status: "idle",
  latest: null,
  asset: null,
  error: null,
};
