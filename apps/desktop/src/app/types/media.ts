export type MediaItem = {
  id: string;
  name: string;
  path: string;
};

export type PlaybackHistoryEntry = {
  path: string;
  name: string;
  position: number;
  duration: number;
  updatedAt: number;
};

export type NetworkStreamHistoryEntry = {
  url: string;
  name: string;
  scheme: string;
  updatedAt: number;
};

export type MpvTrack = {
  id: number;
  kind: "audio" | "video" | "sub";
  title: string | null;
  language: string | null;
  codec: string | null;
  selected: boolean;
  external: boolean;
};

export type MpvSnapshot = {
  path: string;
  status: string;
  ended: boolean;
  paused: boolean;
  position: number;
  duration: number;
  fps: number;
  speed: number;
  hwdec: string;
  videoFill: boolean;
  subtitleDelay: number;
  volume: number;
  tracks: MpvTrack[];
};

export type MpvLoadOptions = Record<string, string>;

export type MpvCaptureArtifact = {
  path: string;
  copiedToClipboard: boolean;
};

export type MpvWallTileRequest = {
  id: string;
  url: string;
  title?: string;
  x: number;
  y: number;
  width: number;
  height: number;
  muted?: boolean;
};

export type MpvWallTileLayout = {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
};

export type MpvWallTileSnapshot = {
  id: string;
  url: string;
  title: string | null;
  status: string;
  latencySeconds: number | null;
  bufferSeconds: number | null;
  bitrateBps: number | null;
  message: string | null;
};
