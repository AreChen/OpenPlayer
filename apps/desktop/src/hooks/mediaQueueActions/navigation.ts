import type { LoopMode } from "../../app/types";

export function previousQueueIndexFor(
  currentIndex: number | null,
  queueLength: number,
  loopMode: LoopMode,
) {
  if (currentIndex === null || queueLength === 0) {
    return null;
  }
  if (currentIndex > 0) {
    return currentIndex - 1;
  }
  return loopMode === "all" && queueLength > 1 ? queueLength - 1 : null;
}

export function nextQueueIndexFor(
  currentIndex: number | null,
  queueLength: number,
  loopMode: LoopMode,
) {
  if (currentIndex === null || queueLength === 0) {
    return null;
  }
  if (currentIndex < queueLength - 1) {
    return currentIndex + 1;
  }
  return loopMode === "all" && queueLength > 1 ? 0 : null;
}
