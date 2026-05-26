import { invoke } from "@tauri-apps/api/core";
import type { AppStrings } from "../i18n";
import { DEFAULT_PLAYBACK_SETTINGS, END_OF_MEDIA_SNAP_TOLERANCE_SECONDS, MIN_RESUME_PROGRESS_RATIO, RESUME_END_PROGRESS_RATIO } from "./constants";
import type { HardwareDecodingMode, LoopMode, MpvTrack, PlatformSupport, PlaybackHistoryEntry, PlaybackSettings, PlaybackSettingsUpdate, TimeDisplayMode } from "./types";

export function loopModeOptionsFor(t: AppStrings): Array<{ mode: LoopMode; label: string; description: string }> {
  return [
    { mode: "off", ...t.loop.off },
    { mode: "one", ...t.loop.one },
    { mode: "all", ...t.loop.all },
  ];
}

export function formatTimecode(value: number, totalDuration: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return totalDuration > 3600 ? "0:00:00" : "00:00";
  }

  const totalSeconds = Math.floor(value);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (totalDuration > 3600) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${Math.floor(totalSeconds / 60).toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

export function resumePositionWithinDuration(position: number, duration: number) {
  if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0 || position <= 0) {
    return 0;
  }

  const clamped = Math.min(position, duration);
  const ratio = clamped / duration;
  if (ratio < MIN_RESUME_PROGRESS_RATIO || ratio >= RESUME_END_PROGRESS_RATIO) {
    return 0;
  }

  return clamped;
}

export async function resumePositionForPath(path: string) {
  try {
    const position = await invoke<number>("history_resume_position", { path });
    return Number.isFinite(position) ? Math.max(0, position) : 0;
  } catch (error) {
    console.warn("Failed to resolve playback resume position", error);
    return 0;
  }
}

export function formatHistoryProgress(entry: PlaybackHistoryEntry, t: AppStrings) {
  if (!Number.isFinite(entry.duration) || entry.duration <= 0) {
    return t.status.noRecordedProgress;
  }

  const resumePosition = resumePositionWithinDuration(entry.position, entry.duration);
  if (resumePosition <= 0) {
    return t.status.playFromStart;
  }

  return `${formatTimecode(resumePosition, entry.duration)} / ${formatTimecode(entry.duration, entry.duration)}`;
}

export function formatFrameCount(value: number, locale: string) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0";
  }

  return Math.floor(value).toLocaleString(locale);
}

export function canDisplayFrames(fps: number, duration: number) {
  return Number.isFinite(fps) && fps > 0 && Number.isFinite(duration) && duration > 0;
}

export function clampPlaybackSpeed(value: number) {
  if (!Number.isFinite(value)) {
    return 1;
  }

  return Math.min(4, Math.max(0.25, value));
}

export function formatPlaybackSpeed(value: number) {
  const speed = clampPlaybackSpeed(value);
  return `${Number.isInteger(speed) ? speed.toFixed(0) : speed.toFixed(2).replace(/0$/, "")}x`;
}

export function loopModeLabel(mode: LoopMode, t: AppStrings) {
  return loopModeOptionsFor(t).find((option) => option.mode === mode)?.label ?? t.loop.off.label;
}

export function clampSubtitleDelay(value: number) {
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.min(10, Math.max(-10, value));
}

export function formatSubtitleDelay(value: number) {
  const delay = clampSubtitleDelay(value);
  if (Math.abs(delay) < 0.005) {
    return "0.0s";
  }

  return `${delay > 0 ? "+" : ""}${delay.toFixed(1)}s`;
}

export function platformUnsupportedPlaybackMessage(support: PlatformSupport | null, t: AppStrings) {
  const session = [support?.os, support?.displayServer].filter(Boolean).join(" / ") || t.common.currentPlatform;
  return t.status.unsupportedPlayback(session);
}

export function trackDisplayLabel(track: MpvTrack, t: AppStrings) {
  const title = track.title || `${track.kind.toUpperCase()} ${track.id}`;
  const details = [track.language?.toUpperCase(), track.codec, track.external ? t.common.external : null].filter(Boolean);
  return details.length ? `${title} · ${details.join(" · ")}` : title;
}

export function snapEndOfMediaPosition(position: number, duration: number, isPlaying: boolean) {
  if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0) {
    return Number.isFinite(position) ? Math.max(0, position) : 0;
  }

  const clamped = Math.min(duration, Math.max(0, position));
  if (!isPlaying && duration - clamped <= END_OF_MEDIA_SNAP_TOLERANCE_SECONDS) {
    return duration;
  }

  return clamped;
}

export function hwdecModeFromSnapshot(hwdec: string | null | undefined): HardwareDecodingMode {
  return hwdec?.trim().toLowerCase() === "no" ? "software" : "hardware";
}

export function normalizeLoopMode(mode: unknown): LoopMode {
  return mode === "one" || mode === "all" ? mode : "off";
}

export function normalizeHardwareDecodingMode(mode: unknown): HardwareDecodingMode {
  return mode === "software" ? "software" : "hardware";
}

export function normalizeTimeDisplayMode(mode: unknown): TimeDisplayMode {
  return mode === "frames" ? "frames" : "timecode";
}

export function normalizePlaybackSettings(settings: Partial<PlaybackSettings> | null | undefined): PlaybackSettings {
  const volume = settings?.volume;
  return {
    volume: Number.isFinite(volume) ? Math.min(100, Math.max(0, volume as number)) : DEFAULT_PLAYBACK_SETTINGS.volume,
    loopMode: normalizeLoopMode(settings?.loopMode),
    hwdecMode: normalizeHardwareDecodingMode(settings?.hwdecMode),
    playbackSpeed: clampPlaybackSpeed(settings?.playbackSpeed ?? DEFAULT_PLAYBACK_SETTINGS.playbackSpeed),
    videoFill: settings?.videoFill === true,
    timeDisplayMode: normalizeTimeDisplayMode(settings?.timeDisplayMode),
  };
}

export function mergePlaybackSettings(current: PlaybackSettings, update: PlaybackSettingsUpdate): PlaybackSettings {
  return normalizePlaybackSettings({ ...current, ...update });
}
