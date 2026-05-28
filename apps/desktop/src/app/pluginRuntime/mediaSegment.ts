import type { MediaItem } from "../types";

const DEFAULT_PLUGIN_MEDIA_SEGMENT_SECONDS = 10;
const MIN_PLUGIN_MEDIA_SEGMENT_SECONDS = 0.25;
const MAX_PLUGIN_MEDIA_SEGMENT_SECONDS = 120;
const DEFAULT_PLUGIN_MEDIA_TIMELINE_MAX_SEGMENTS = 1000;
const MAX_PLUGIN_MEDIA_TIMELINE_SEGMENTS = 2000;

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

export type PluginMediaSegmentTimelineInput = {
  start?: number;
  end?: number;
  duration?: number;
  overlap?: number;
  maxSegments?: number;
};

export type PluginMediaSegmentTimeline = {
  media: MediaItem;
  position: number;
  mediaDuration: number;
  start: number;
  end: number;
  duration: number;
  segmentDuration: number;
  overlap: number;
  truncated: boolean;
  segments: PluginMediaSegment[];
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

function boundedTimelineMaxSegments(value: number | undefined) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return DEFAULT_PLUGIN_MEDIA_TIMELINE_MAX_SEGMENTS;
  }
  return Math.min(MAX_PLUGIN_MEDIA_TIMELINE_SEGMENTS, Math.max(1, Math.floor(value)));
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

export function normalizePluginMediaSegmentTimeline(
  input: PluginMediaSegmentTimelineInput,
  context: PluginMediaSegmentContext,
): PluginMediaSegmentTimeline {
  if (!context.media) {
    throw new Error("media.segmentTimeline requires loaded media");
  }

  const position = finiteNonNegative(context.position);
  const mediaDuration = finiteNonNegative(context.mediaDuration);
  if (mediaDuration <= 0) {
    throw new Error("media.segmentTimeline requires known media duration");
  }

  const segmentDuration = boundedDuration(input.duration);
  const overlap = Math.min(segmentDuration - MIN_PLUGIN_MEDIA_SEGMENT_SECONDS, finiteNonNegative(input.overlap ?? 0));
  const maxSegments = boundedTimelineMaxSegments(input.maxSegments);
  const start = Math.min(finiteNonNegative(input.start ?? 0), mediaDuration);
  const explicitEnd = typeof input.end === "number" && Number.isFinite(input.end) ? input.end : mediaDuration;
  const end = Math.min(mediaDuration, Math.max(start, finiteNonNegative(explicitEnd, mediaDuration)));
  if (end <= start) {
    throw new Error("media.segmentTimeline could not produce a non-empty timeline");
  }

  const step = segmentDuration - overlap;
  const segments: PluginMediaSegment[] = [];
  let cursor = start;
  while (cursor < end && segments.length < maxSegments) {
    const segmentEnd = Math.min(end, cursor + segmentDuration);
    segments.push({
      media: context.media,
      position,
      mediaDuration,
      start: cursor,
      end: segmentEnd,
      duration: segmentEnd - cursor,
      clip: {
        start: cursor,
        duration: segmentEnd - cursor,
      },
    });
    if (segmentEnd >= end) {
      break;
    }
    cursor += step;
  }

  if (segments.length === 0) {
    throw new Error("media.segmentTimeline could not produce a non-empty timeline");
  }

  return {
    media: context.media,
    position,
    mediaDuration,
    start,
    end,
    duration: end - start,
    segmentDuration,
    overlap,
    truncated: segments[segments.length - 1]?.end < end,
    segments,
  };
}
