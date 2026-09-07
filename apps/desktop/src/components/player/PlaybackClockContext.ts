import { createContext, useCallback, useContext, useSyncExternalStore } from "react";
import type { PlaybackClock } from "../../app/playbackClock";

export const PlaybackClockContext = createContext<{
  clock: PlaybackClock;
  timelineVisible: boolean;
  framesPerSecond: number;
  locale: string;
} | null>(null);

export function usePlaybackPosition(fallback: number, enabled = true) {
  const context = useContext(PlaybackClockContext);
  const clock = context?.clock;
  const subscribe = useCallback((listener: () => void) => {
    return enabled && clock ? clock.subscribe(listener) : () => {};
  }, [clock, enabled]);
  const getSnapshot = useCallback(() => clock?.getSnapshot() ?? fallback, [clock, fallback]);
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
