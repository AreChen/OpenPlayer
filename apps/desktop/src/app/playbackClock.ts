import { clampPlaybackSpeed, snapEndOfMediaPosition } from "./playback";

export function createPlaybackClock() {
  let anchor = { position: 0, startedAt: performance.now(), playing: false, duration: 0, speed: 1 };
  let snapshot = 0;
  let active = false;
  let frame: number | null = null;
  const listeners = new Set<() => void>();

  function getPosition() {
    const elapsed = anchor.playing ? (performance.now() - anchor.startedAt) / 1000 : 0;
    const position = Math.max(0, anchor.position + elapsed * anchor.speed);
    return snapEndOfMediaPosition(
      anchor.duration > 0 ? Math.min(anchor.duration, position) : position,
      anchor.duration,
      anchor.playing,
    );
  }
  function publish() {
    const next = getPosition();
    if (next === snapshot) return;
    snapshot = next;
    listeners.forEach((listener) => listener());
  }
  function schedule() {
    if (frame !== null) window.cancelAnimationFrame(frame);
    frame = null;
    if (!active || !anchor.playing || !listeners.size || document.hidden) return;
    frame = window.requestAnimationFrame(() => {
      frame = null;
      publish();
      schedule();
    });
  }
  function visibilityChanged() {
    if (!document.hidden) publish();
    schedule();
  }
  return {
    getPosition,
    getSnapshot: () => snapshot,
    subscribe(listener: () => void) {
      listeners.add(listener);
      if (listeners.size === 1) document.addEventListener("visibilitychange", visibilityChanged);
      publish();
      schedule();
      return () => {
        listeners.delete(listener);
        if (!listeners.size) document.removeEventListener("visibilitychange", visibilityChanged);
        schedule();
      };
    },
    setActive(value: boolean) {
      active = value;
      schedule();
    },
    anchor(position: number, playing: boolean, duration: number, speed: number) {
      const safePosition = Number.isFinite(position) ? Math.max(0, position) : 0;
      anchor = {
        position: duration > 0 ? Math.min(duration, safePosition) : safePosition,
        startedAt: performance.now(), playing, duration, speed: clampPlaybackSpeed(speed),
      };
      publish();
      schedule();
    },
  };
}

export type PlaybackClock = ReturnType<typeof createPlaybackClock>;
