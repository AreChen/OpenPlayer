import type { AppStrings } from "../i18n";
import { OPENPLAYER_RELEASES_URL } from "./constants";
import type { LatestRelease, PlatformSupport, ReleaseAsset, UpdateState } from "./types";

function normalizeReleaseVersion(value: string) {
  return value.trim().replace(/^v/i, "");
}

export function compareVersionParts(current: string, latest: string) {
  const left = normalizeReleaseVersion(current).split(".").map((part) => Number.parseInt(part, 10));
  const right = normalizeReleaseVersion(latest).split(".").map((part) => Number.parseInt(part, 10));
  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const leftValue = left[index];
    const rightValue = right[index];
    const leftPart = typeof leftValue === "number" && Number.isFinite(leftValue) ? leftValue : 0;
    const rightPart = typeof rightValue === "number" && Number.isFinite(rightValue) ? rightValue : 0;
    if (leftPart !== rightPart) {
      return leftPart < rightPart ? -1 : 1;
    }
  }
  return 0;
}

function normalizeReleaseAsset(asset: unknown): ReleaseAsset | null {
  if (!asset || typeof asset !== "object") {
    return null;
  }

  const record = asset as Record<string, unknown>;
  if (typeof record.name !== "string" || typeof record.browser_download_url !== "string") {
    return null;
  }

  return {
    name: record.name,
    browserDownloadUrl: record.browser_download_url,
  };
}

export function normalizeLatestRelease(payload: unknown): LatestRelease | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const record = payload as Record<string, unknown>;
  const tagName = typeof record.tag_name === "string" ? record.tag_name : "";
  const htmlUrl = typeof record.html_url === "string" ? record.html_url : OPENPLAYER_RELEASES_URL;
  const assets = Array.isArray(record.assets) ? record.assets.map(normalizeReleaseAsset).filter((asset): asset is ReleaseAsset => asset !== null) : [];
  const version = normalizeReleaseVersion(tagName);
  if (!version) {
    return null;
  }

  return { version, tagName, htmlUrl, assets };
}

function releaseAssetCandidates(latest: LatestRelease, support: PlatformSupport | null) {
  const version = latest.version;
  const os = support?.os.toLowerCase() ?? "";
  const userAgent = navigator.userAgent.toLowerCase();
  if (os === "windows") {
    return [`OpenPlayer_${latest.version}_x64-setup.exe`];
  }
  if (os === "macos") {
    const macosAssets = [`OpenPlayer_${version}_x64.dmg`, `OpenPlayer_${version}_arm64.dmg`];
    return userAgent.includes("arm") || userAgent.includes("aarch64") ? [...macosAssets].reverse() : macosAssets;
  }
  if (os === "linux") {
    return [`OpenPlayer_${version}_amd64.AppImage`, `OpenPlayer_${version}_amd64.deb`];
  }
  return [`OpenPlayer_${version}_x64-setup.exe`, `OpenPlayer_${version}_amd64.AppImage`, `OpenPlayer_${version}_x64.dmg`, `OpenPlayer_${version}_arm64.dmg`];
}

export function releaseAssetForCurrentPlatform(latest: LatestRelease, support: PlatformSupport | null) {
  const candidates = releaseAssetCandidates(latest, support);
  for (const candidate of candidates) {
    const asset = latest.assets.find((item) => item.name === candidate);
    if (asset) {
      return asset;
    }
  }
  return latest.assets.find((asset) => asset.name.includes(`OpenPlayer_${latest.version}_`) && !asset.name.endsWith(".sha256")) ?? null;
}

export function updateStatusText(state: UpdateState, t: AppStrings) {
  switch (state.status) {
    case "checking":
      return t.settings.about.checkingUpdates;
    case "current":
      return t.settings.about.upToDate;
    case "available":
      return state.latest ? t.settings.about.updateAvailable(state.latest.version) : t.settings.about.updateAvailableUnknown;
    case "failed":
      return t.settings.about.updateCheckFailed(state.error ?? t.common.none);
    case "idle":
    default:
      return t.settings.about.updateIdle;
  }
}
