import type { AppStrings } from "../i18n";
import { OPENPLAYER_RELEASES_URL } from "./constants";
import type { LatestRelease, PlatformSupport, ReleaseAsset, UpdateState } from "./types";

function normalizeReleaseVersion(value: string) {
  const version = value.replace(/^v/, "");
  return /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)
    && version.split(".").every((part) => Number(part) <= 0xffffffff) ? version : null;
}

export function compareVersionParts(current: string, latest: string) {
  const left = current.split(".").map(Number);
  const right = latest.split(".").map(Number);
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

function normalizeReleaseAsset(asset: unknown, tagName: string): ReleaseAsset | null {
  if (!asset || typeof asset !== "object") {
    return null;
  }

  const record = asset as Record<string, unknown>;
  if (record.state !== "uploaded" || typeof record.name !== "string" || !/^[\w.-]+$/.test(record.name)) {
    return null;
  }

  return {
    name: record.name,
    browserDownloadUrl: `${OPENPLAYER_RELEASES_URL}/download/${tagName}/${record.name}`,
  };
}

function normalizeLatestRelease(payload: unknown): LatestRelease | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const record = payload as Record<string, unknown>;
  const tagName = typeof record.tag_name === "string" ? record.tag_name : "";
  const version = normalizeReleaseVersion(tagName);
  if (!version || record.draft !== false || record.prerelease !== false) {
    return null;
  }

  const htmlUrl = `${OPENPLAYER_RELEASES_URL}/tag/${tagName}`;
  const assets = Array.isArray(record.assets)
    ? record.assets.map((asset) => normalizeReleaseAsset(asset, tagName)).filter((asset): asset is ReleaseAsset => asset !== null)
    : [];

  return { version, tagName, htmlUrl, assets };
}

function releaseAssetCandidates(latest: LatestRelease, support: PlatformSupport | null) {
  const version = latest.version;
  const userAgent = navigator.userAgent.toLowerCase();
  const os = support?.os.toLowerCase()
    ?? (userAgent.includes("windows") ? "windows" : userAgent.includes("macintosh") ? "macos" : userAgent.includes("linux") ? "linux" : "");
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
  return [];
}

function releaseAssetForCurrentPlatform(latest: LatestRelease, support: PlatformSupport | null) {
  const candidates = releaseAssetCandidates(latest, support);
  for (const candidate of candidates) {
    const asset = latest.assets.find((item) => item.name === candidate);
    if (asset) {
      return asset;
    }
  }
  return null;
}

export function latestInstallableRelease(releases: unknown[], installedVersion: string, support: PlatformSupport | null) {
  const installed = normalizeReleaseVersion(installedVersion);
  if (!installed) throw new Error("invalid installed version");

  const [major] = installed.split(".");
  let update: { latest: LatestRelease; asset: ReleaseAsset } | null = null;
  for (const payload of releases) {
    const latest = normalizeLatestRelease(payload);
    if (!latest || !latest.version.startsWith(`${major}.`) || compareVersionParts(update?.latest.version ?? installed, latest.version) >= 0) {
      continue;
    }
    const asset = releaseAssetForCurrentPlatform(latest, support);
    if (asset) update = { latest, asset };
  }
  return update;
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
