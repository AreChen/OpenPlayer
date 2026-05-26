export type PlatformSupport = {
  os: string;
  displayServer: string;
  mpvEmbedVideo: boolean;
  nativeShortcutBridge: boolean;
};

export type ShellPreviewRegistrationSummary = {
  registeredCount: number;
  videoCount: number;
  audioCount: number;
  extensions: string[];
};

export type ShellPreviewFormatInfo = {
  extension: string;
  mime: string;
  kind: "video" | "audio";
  common: boolean;
};

export type AppVersionInfo = {
  name: string;
  version: string;
  license: string;
  repository: string;
  releasesUrl: string;
};

export type ReleaseAsset = {
  name: string;
  browserDownloadUrl: string;
};

export type LatestRelease = {
  version: string;
  tagName: string;
  htmlUrl: string;
  assets: ReleaseAsset[];
};

export type UpdateState = {
  status: "idle" | "checking" | "current" | "available" | "failed";
  latest: LatestRelease | null;
  asset: ReleaseAsset | null;
  error: string | null;
};

export type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_toggle_fullscreen" | "window_close";
