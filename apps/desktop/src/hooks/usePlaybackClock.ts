import { useEffect, useRef, useState } from "react";
import { clampPlaybackSpeed } from "../app/playback";
import type { PlaybackClockAnchor } from "../app/types";

type UsePlaybackClockOptions = {
  mediaId: string | null | undefined;
  isPlaying: boolean;
  duration: number;
  playbackSpeed: number;
};

export function usePlaybackClock({ mediaId, isPlaying, duration, playbackSpeed }: UsePlaybackClockOptions) {
  const [displayPosition, setDisplayPosition] = useState(0);
  const playbackClockAnchorRef = useRef<PlaybackClockAnchor>({ position: 0, startedAt: performance.now(), playing: false, speed: 1 });

  function clampPlaybackPosition(value: number, upperDuration = duration) {
    if (!Number.isFinite(value)) {
      return 0;
    }

    const upperBound = upperDuration > 0 ? upperDuration : value;
    return Math.min(upperBound, Math.max(0, value));
  }

  function anchorDisplayClock(position: number, playing: boolean, upperDuration = duration, speed = playbackSpeed) {
    const clampedPosition = clampPlaybackPosition(position, upperDuration);
    playbackClockAnchorRef.current = {
      position: clampedPosition,
      startedAt: performance.now(),
      playing,
      speed: clampPlaybackSpeed(speed),
    };
    setDisplayPosition(clampedPosition);
  }

  useEffect(() => {
    if (!mediaId || !isPlaying || duration <= 0) {
      return;
    }

    let frameId = 0;
    const tick = () => {
      const anchor = playbackClockAnchorRef.current;
      const elapsedSeconds = anchor.playing ? (performance.now() - anchor.startedAt) / 1000 : 0;
      setDisplayPosition(clampPlaybackPosition(anchor.position + elapsedSeconds * anchor.speed, duration));
      frameId = window.requestAnimationFrame(tick);
    };

    frameId = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frameId);
  }, [mediaId, isPlaying, duration]);

  return {
    displayPosition,
    anchorDisplayClock,
  };
}
