import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_UPDATE_STATE, OPENPLAYER_RELEASES_API_URL, OPENPLAYER_RELEASES_URL } from "../app/constants";
import { compareVersionParts, latestInstallableRelease } from "../app/updates";
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
    const timeout = new AbortController();
    const timer = window.setTimeout(() => timeout.abort(), 20_000);
    try {
      const versionInfo = appVersion ?? (await invoke<AppVersionInfo>("app_version"));
      if (!appVersion) {
        onSetAppVersion(versionInfo);
      }

      const pageSize = 100;
      let update: ReturnType<typeof latestInstallableRelease> = null;
      for (let page = 1; ; page += 1) {
        const response = await fetch(`${OPENPLAYER_RELEASES_API_URL}?per_page=${pageSize}&page=${page}`, {
          headers: { Accept: "application/vnd.github+json" },
          signal: timeout.signal,
        });
        if (!response.ok) throw new Error(`GitHub ${response.status}`);
        const releases: unknown = await response.json();
        if (!Array.isArray(releases)) throw new Error("invalid release list");
        const candidate = latestInstallableRelease(releases, versionInfo.version, platformSupport);
        if (candidate && (!update || compareVersionParts(update.latest.version, candidate.latest.version) < 0)) update = candidate;
        if (releases.length < pageSize) break;
      }
      setUpdateState({
        status: update ? "available" : "current",
        latest: update?.latest ?? null,
        asset: update?.asset ?? null,
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
      window.clearTimeout(timer);
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
