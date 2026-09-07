import assert from "node:assert/strict";
import test from "node:test";
import { loadSource } from "./sourceHarness.mjs";

function setup() {
  let now = 0;
  let sequence = 0;
  const frames = new Map();
  const events = new Map();
  const document = {
    hidden: false,
    addEventListener: (name, callback) => events.set(name, callback),
    removeEventListener: (name) => events.delete(name),
  };
  const constants = loadSource("src/app/constants/timing.ts");
  const playback = loadSource("src/app/playback.ts", { "./constants": constants, "@tauri-apps/api/core": {} });
  const { createPlaybackClock } = loadSource("src/app/playbackClock.ts", { "./playback": playback }, {
    performance: { now: () => now }, document,
    window: {
      requestAnimationFrame(callback) { frames.set(++sequence, callback); return sequence; },
      cancelAnimationFrame(id) { frames.delete(id); },
    },
  });
  return {
    clock: createPlaybackClock(), frames, document, events,
    advance(ms) { now += ms; const pending = [...frames.values()]; frames.clear(); pending.forEach((callback) => callback()); },
  };
}

test("clock interpolation stays local to subscribers and does no frame work without them", () => {
  const { clock, frames, advance } = setup();
  clock.anchor(10, true, 100, 1);
  clock.setActive(true);
  assert.equal(frames.size, 0);
  advance(1000);
  assert.equal(clock.getPosition(), 11);
  let renders = 0;
  const unsubscribe = clock.subscribe(() => renders++);
  advance(1000);
  assert.equal(clock.getSnapshot(), 12);
  assert.ok(renders > 0);
  unsubscribe();
  assert.equal(frames.size, 0);
  advance(1000);
  assert.equal(clock.getPosition(), 13);
});

test("pause, seek, speed and duration clamping preserve authoritative anchors", () => {
  const { clock, advance } = setup();
  const unsubscribe = clock.subscribe(() => {});
  clock.setActive(true);
  clock.anchor(20, true, 100, 2);
  advance(2000);
  assert.equal(clock.getPosition(), 24);
  clock.anchor(24, false, 100, 2);
  advance(2000);
  assert.equal(clock.getSnapshot(), 24);
  clock.anchor(99, true, 100, 1);
  advance(5000);
  assert.equal(clock.getSnapshot(), 100);
  clock.anchor(0, false, 0, 1);
  assert.equal(clock.getSnapshot(), 0);
  unsubscribe();
});

test("hidden documents stop scheduling and catch up when made visible", () => {
  const { clock, frames, document, events, advance } = setup();
  clock.anchor(0, true, 100, 1);
  clock.setActive(true);
  const unsubscribe = clock.subscribe(() => {});
  document.hidden = true;
  events.get("visibilitychange")();
  assert.equal(frames.size, 0);
  advance(3000);
  document.hidden = false;
  events.get("visibilitychange")();
  assert.equal(clock.getSnapshot(), 3);
  assert.equal(frames.size, 1);
  unsubscribe();
  assert.equal(events.size, 0);
});
