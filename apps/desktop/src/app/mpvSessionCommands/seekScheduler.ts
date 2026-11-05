const SEEK_INTERVAL_MS = 120;

export function createSeekScheduler() {
  let pending: (() => Promise<void>) | null = null;
  let running = false;
  let lastStarted = Number.NEGATIVE_INFINITY;
  let timer: ReturnType<typeof setTimeout> | null = null;

  function pump() {
    if (running || !pending || timer !== null) return;
    const delay = Math.max(0, lastStarted + SEEK_INTERVAL_MS - performance.now());
    if (delay > 0) {
      timer = setTimeout(() => { timer = null; pump(); }, delay);
      return;
    }
    const work = pending;
    pending = null;
    running = true;
    lastStarted = performance.now();
    const done = () => { running = false; pump(); };
    try { void Promise.resolve(work()).then(done, done); } catch { done(); }
  }

  return {
    enqueue(work: () => Promise<void>) { pending = work; pump(); },
    cancel() {
      pending = null;
      if (timer !== null) clearTimeout(timer);
      timer = null;
      lastStarted = Number.NEGATIVE_INFINITY;
    },
  };
}
