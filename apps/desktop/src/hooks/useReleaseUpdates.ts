import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_UPDATE_STATE, OPENPLAYER_RELEASES_API_URL, OPENPLAYER_RELEASES_URL } from "../app/constants";
import { compareVersionParts, normalizeLatestRelease, releaseAssetForCurrentPlatform } from "../app/updates";
import type { AppVersionInfo, PlatformSupport, UpdateState } from "../app/types";

type UseReleaseUpdatesOptions = {
  appVersion: AppVersionInfo | null;
  platformSupport: PlatformSupport | null;
  onSetAppVersion: (version: AppVersionInfo) => void;
  onOpenExternalUrl: (url: string) => void;
  onCheckSettled: () => void;
};

export function useReleaseUpdates({ appVersion, platformSupport, onSetAppVersion, onOpenExternalUrl, onCheckSettled }: UseReleaseUpdatesOptions) {
  const [updateState, setUpdateState] = useState<UpdateState>(DEFAULT_UPDATE_STATE);

  async function checkForUpdates() {
    if (updateState.status === "checking") {
      return;
    }

    setUpdateState((state) => ({ ...state, status: "checking", error: null }));
    try {
      const response = await fetch(OPENPLAYER_RELEASES_API_URL, {
        headers: { Accept: "application/vnd.github+json" },
      });
      if (!response.ok) {
        throw new Error(`GitHub ${response.status}`);
      }

      const latest = normalizeLatestRelease(await response.json());
      if (!latest) {
        throw new Error("invalid release response");
      }

      const versionInfo = appVersion ?? (await invoke<AppVersionInfo>("app_version"));
      if (!appVersion) {
        onSetAppVersion(versionInfo);
      }
      const asset = releaseAssetForCurrentPlatform(latest, platformSupport);
      setUpdateState({
        status: compareVersionParts(versionInfo.version, latest.version) < 0 ? "available" : "current",
        latest,
        asset,
        error: null,
      });
    } catch (error) {
      setUpdateState({
        status: "failed",
        latest: null,
        asset: null,
        error: error instanceof Error ? error.message : String(error),
      });
    } finally {
      onCheckSettled();
    }
  }

  function openUpdateDownload() {
    if (updateState.status === "available" && updateState.asset) {
      onOpenExternalUrl(updateState.asset.browserDownloadUrl);
      return;
    }

    if (updateState.latest) {
      onOpenExternalUrl(updateState.latest.htmlUrl);
      return;
    }

    onOpenExternalUrl(appVersion?.releasesUrl ?? OPENPLAYER_RELEASES_URL);
  }

  return {
    updateState,
    checkForUpdates,
    openUpdateDownload,
  };
}
