export type LoopMode = "off" | "one" | "all";
export type HardwareDecodingMode = "hardware" | "software";
export type TimeDisplayMode = "timecode" | "frames";
export type SelectableTrackKind = "audio" | "video" | "subtitle";

export type PlaybackSettings = {
  volume: number;
  loopMode: LoopMode;
  hwdecMode: HardwareDecodingMode;
  playbackSpeed: number;
  videoFill: boolean;
  timeDisplayMode: TimeDisplayMode;
};

export type PlaybackSettingsUpdate = Partial<PlaybackSettings>;

export type MediaPlaybackSettings = {
  path: string;
  subtitleTrackId: number | null;
  hasSubtitleTrackSelection: boolean;
};

export type PendingSeek = {
  target: number;
  startedAt: number;
};

export type PlaybackClockAnchor = {
  position: number;
  startedAt: number;
  playing: boolean;
  speed: number;
};

export type VolumeFeedback = {
  level: number;
};

export type AlwaysOnTopFeedback = {
  enabled: boolean;
};

export type CaptureFeedback = {
  icon: "camera" | "record" | "info";
  message: string;
};

export type MpvRecordingState = {
  active: boolean;
  path: string | null;
  format: string | null;
};
