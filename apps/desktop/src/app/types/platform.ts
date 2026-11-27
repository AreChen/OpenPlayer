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

export type UpdateOffer = {
  version: string;
  package: "dmg" | "deb" | "appImage";
};

export type UpdateState =
  | { status: "idle" | "checking" | "current" | "unsupported" }
  | { status: "available" | "downloading" | "ready" | "installing" | "installed" | "installerOpened"; update: UpdateOffer }
  | { status: "checkFailed"; message: string }
  | { status: "downloadFailed" | "installFailed" | "restartFailed"; update: UpdateOffer; message: string };

export type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_toggle_fullscreen" | "window_close";
