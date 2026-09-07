import { useEffect, useRef } from "react";
import { createPlaybackClock } from "../app/playbackClock";

type UsePlaybackClockOptions = {
  mediaId: string | null | undefined;
  isPlaying: boolean;
  duration: number;
  playbackSpeed: number;
};

export function usePlaybackClock({ mediaId, isPlaying, duration, playbackSpeed }: UsePlaybackClockOptions) {
  const clockRef = useRef<ReturnType<typeof createPlaybackClock> | null>(null);
  if (!clockRef.current) clockRef.current = createPlaybackClock();
  const clock = clockRef.current;

  useEffect(() => {
    clock.setActive(Boolean(mediaId && isPlaying && duration > 0));
    if (!mediaId) clock.anchor(0, false, 0, 1);
    return () => clock.setActive(false);
  }, [clock, mediaId, isPlaying, duration]);

  return {
    clock,
    displayPosition: clock.getPosition(),
    getDisplayTime: clock.getPosition,
    anchorDisplayClock: (position: number, playing: boolean, upperDuration = duration, speed = playbackSpeed) => {
      clock.anchor(position, playing, upperDuration, speed);
    },
  };
}
