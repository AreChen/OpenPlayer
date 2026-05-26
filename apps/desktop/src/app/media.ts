import { audioOnlyExtensions, mediaPathCollator, playableExtensions } from "./constants";
import type { MediaItem, PlaybackHistoryEntry, ShellPreviewFormatInfo } from "./types";

let mediaItemIdCounter = 0;

export function nextMediaItemId() {
  mediaItemIdCounter += 1;
  return `path:${mediaItemIdCounter}`;
}

export function mediaNameFromPath(path: string) {
  const normalized = path.replace(/\\/g, "/");
  return normalized.split("/").pop() || path;
}

export function parentDirectoryFromPath(path: string | null | undefined) {
  if (!path) {
    return null;
  }
  const normalized = path.replace(/\\/g, "/");
  const index = normalized.lastIndexOf("/");
  if (index <= 0) {
    return null;
  }
  return path.slice(0, index);
}

export function streamNameFromUrl(url: string, fallbackName: string | null = null) {
  if (fallbackName?.trim()) {
    return fallbackName.trim();
  }
  try {
    const parsed = new URL(url);
    const pathName = decodeURIComponent(parsed.pathname.split("/").filter(Boolean).pop() ?? "");
    return pathName || parsed.hostname || url;
  } catch {
    return mediaNameFromPath(url);
  }
}

const networkStreamProtocols = new Set(["http:", "https:", "rtmp:", "rtmps:", "rtsp:", "rtsps:"]);

export function normalizeNetworkStreamInput(url: string) {
  const trimmed = url.trim();
  if (!trimmed || /\s/.test(trimmed) || trimmed.length > 2048) {
    throw new Error("network stream url is invalid");
  }
  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    throw new Error("network stream url must include a protocol");
  }
  if (!networkStreamProtocols.has(parsed.protocol.toLowerCase())) {
    throw new Error(`unsupported network stream protocol: ${parsed.protocol.replace(":", "")}`);
  }
  if (!parsed.host && !parsed.pathname.replace(/\//g, "")) {
    throw new Error("network stream url must include a host or path");
  }
  parsed.protocol = parsed.protocol.toLowerCase();
  return parsed.toString();
}

export function defaultShellPreviewExtensions(formats: ShellPreviewFormatInfo[]) {
  return formats.filter((format) => format.common).map((format) => format.extension);
}

export function isPlayableMediaPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return Boolean(extension && playableExtensions.includes(extension));
}

export function isAudioMediaPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return Boolean(extension && audioOnlyExtensions.includes(extension));
}

export function isOpenPlayerPluginPackagePath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return extension === "opplugin";
}

export function sortMediaPaths(paths: string[]) {
  return [...paths]
    .filter(isPlayableMediaPath)
    .sort((left, right) => mediaPathCollator.compare(mediaNameFromPath(left), mediaNameFromPath(right)) || mediaPathCollator.compare(left, right));
}

export function uniqueMediaPaths(paths: string[], existingPaths: Set<string> = new Set()) {
  const seen = new Set(existingPaths);
  const unique: string[] = [];
  for (const path of sortMediaPaths(paths)) {
    if (seen.has(path)) {
      continue;
    }
    seen.add(path);
    unique.push(path);
  }
  return unique;
}

export function mediaItemFromPath(path: string): MediaItem {
  return {
    id: nextMediaItemId(),
    name: mediaNameFromPath(path),
    path,
  };
}

export function isMediaStreamPath(path: string) {
  return /^[a-z][a-z0-9+.-]*:\/\//i.test(path);
}

export function mediaItemFromHistory(entry: PlaybackHistoryEntry): MediaItem {
  return {
    id: nextMediaItemId(),
    name: entry.name || mediaNameFromPath(entry.path),
    path: entry.path,
  };
}
