import assert from "node:assert/strict";
import test from "node:test";
import { deferred, loadSource, settle } from "./sourceHarness.mjs";

function schedulerHarness() {
  let now = 0, nextId = 0;
  const timers = new Map();
  const { createSeekScheduler } = loadSource("src/app/mpvSessionCommands/seekScheduler.ts", {}, {
    performance: { now: () => now },
    setTimeout(callback, delay) { const id = ++nextId; timers.set(id, { callback, at: now + delay }); return id; },
    clearTimeout(id) { timers.delete(id); },
  });
  return {
    scheduler: createSeekScheduler(),
    advance(milliseconds) {
      now += milliseconds;
      for (const [id, timer] of timers) {
        if (timer.at <= now) { timers.delete(id); timer.callback(); }
      }
    },
  };
}

test("rapid seeks start immediately, then coalesce to the last target", async () => {
  const { scheduler, advance } = schedulerHarness();
  const first = deferred();
  const calls = [];
  scheduler.enqueue(() => { calls.push(10); return first.promise; });
  scheduler.enqueue(async () => { calls.push(20); });
  scheduler.enqueue(async () => { calls.push(30); });
  assert.deepEqual(calls, [10]);
  first.resolve();
  await settle();
  advance(119);
  assert.deepEqual(calls, [10]);
  advance(1);
  await settle();
  assert.deepEqual(calls, [10, 30]);
});

test("slow seeks never overlap, and cancellation drops queued targets", async () => {
  const { scheduler, advance } = schedulerHarness();
  const first = deferred();
  const calls = [];
  scheduler.enqueue(() => { calls.push(10); return first.promise; });
  scheduler.enqueue(async () => { calls.push(20); });
  advance(500);
  assert.deepEqual(calls, [10]);
  scheduler.cancel();
  first.resolve();
  await settle();
  advance(500);
  assert.deepEqual(calls, [10]);
  scheduler.enqueue(async () => { calls.push(30); });
  await settle();
  assert.deepEqual(calls, [10, 30]);
});

test("a rejected seek does not block the latest request", async () => {
  const { scheduler, advance } = schedulerHarness();
  const first = deferred();
  const calls = [];
  scheduler.enqueue(() => { calls.push(10); return first.promise; });
  scheduler.enqueue(async () => { calls.push(20); });
  first.reject(new Error("failed"));
  await settle();
  advance(120);
  await settle();
  assert.deepEqual(calls, [10, 20]);
});

test("superseded seek responses and errors cannot overwrite the latest preview", async () => {
  const requests = [];
  const { commitMpvSeek, previewMpvSeek } = loadSource("src/app/mpvSessionCommands/seek.ts", {
    "@tauri-apps/api/core": { invoke: () => { const request = deferred(); requests.push(request); return request.promise; } },
  }, { performance: { now: () => 100 } });
  const applied = [], errors = [];
  const pendingSeekRef = { current: null };
  const options = {
    value: 10, duration: 100, pendingSeekRef, setCurrentTime() {}, anchorDisplayClock() {},
    invalidatePendingSnapshots() {}, applyCommandSnapshot: (snapshot) => applied.push(snapshot), onError: (error) => errors.push(error),
  };
  const first = commitMpvSeek(options);
  previewMpvSeek({ ...options, value: 20 });
  requests[0].resolve({ position: 10 });
  await first;
  assert.equal(pendingSeekRef.current.target, 20);
  assert.deepEqual(applied, []);
  const second = commitMpvSeek({ ...options, value: 20 });
  previewMpvSeek({ ...options, value: 30 });
  requests[1].reject(new Error("old seek failed"));
  await second;
  assert.equal(pendingSeekRef.current.target, 30);
  assert.deepEqual(errors, []);
  const third = commitMpvSeek({ ...options, value: 30 });
  requests[2].resolve({ position: 30 });
  await third;
  assert.equal(applied[0].position, 30);
});
