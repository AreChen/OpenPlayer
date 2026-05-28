import type { MediaItem } from "../types";

const DEFAULT_PLUGIN_MEDIA_SEGMENT_SECONDS = 10;
const MIN_PLUGIN_MEDIA_SEGMENT_SECONDS = 0.25;
const MAX_PLUGIN_MEDIA_SEGMENT_SECONDS = 120;

export type PluginMediaSegmentInput = {
  start?: number;
  before?: number;
  duration?: number;
};

export type PluginMediaSegment = {
  media: MediaItem;
  position: number;
  mediaDuration: number;
  start: number;
  end: number;
  duration: number;
  clip: {
    start: number;
    duration: number;
  };
};

type PluginMediaSegmentContext = {
  media: MediaItem | null;
  position: number;
  mediaDuration: number;
};

function finiteNonNegative(value: number, fallback = 0) {
  return Number.isFinite(value) ? Math.max(0, value) : fallback;
}

function boundedDuration(value: number | undefined) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_PLUGIN_MEDIA_SEGMENT_SECONDS;
  }
  return Math.min(MAX_PLUGIN_MEDIA_SEGMENT_SECONDS, Math.max(MIN_PLUGIN_MEDIA_SEGMENT_SECONDS, value));
}

export function normalizePluginMediaSegment(input: PluginMediaSegmentInput, context: PluginMediaSegmentContext): PluginMediaSegment {
  if (!context.media) {
    throw new Error("media.currentSegment requires loaded media");
  }

  const position = finiteNonNegative(context.position);
  const mediaDuration = finiteNonNegative(context.mediaDuration);
  const requestedDuration = boundedDuration(input.duration);
  const before = Math.min(MAX_PLUGIN_MEDIA_SEGMENT_SECONDS, finiteNonNegative(input.before ?? 0));
  const explicitStart = typeof input.start === "number" && Number.isFinite(input.start) ? input.start : null;
  let start = finiteNonNegative(explicitStart ?? position - before);
  let end = start + requestedDuration;

  if (mediaDuration > 0) {
    start = Math.min(start, mediaDuration);
    end = Math.min(end, mediaDuration);
    if (end <= start) {
      const windowDuration = Math.min(requestedDuration, mediaDuration);
      start = Math.max(0, mediaDuration - windowDuration);
      end = mediaDuration;
    }
  }

  const duration = Math.max(0, end - start);
  if (duration <= 0) {
    throw new Error("media.currentSegment could not produce a non-empty segment");
  }

  return {
    media: context.media,
    position,
    mediaDuration,
    start,
    end,
    duration,
    clip: {
      start,
      duration,
    },
  };
}
