import assert from "node:assert/strict";
import test from "node:test";
import { deferred, hookHarness, loadSource, settle } from "./sourceHarness.mjs";

function setup() {
  const harness = hookHarness();
  const requests = [];
  const applied = [];
  const { useMpvSnapshotPolling } = loadSource("src/hooks/useMpvSnapshotPolling.ts", {
    react: harness.react,
    "@tauri-apps/api/core": { invoke: () => { const request = deferred(); requests.push(request); return request.promise; } },
  }, { window: harness.window });
  const render = (mediaId = "first") => harness.render(() => useMpvSnapshotPolling({ mediaId, applySnapshot: (snapshot) => applied.push(snapshot) }));
  return { harness, requests, applied, render };
}

test("slow snapshots remain applicable and cannot accumulate overlapping requests", async () => {
  const { harness, requests, applied, render } = setup();
  render();
  for (let i = 0; i < 10; i++) harness.tick();
  assert.equal(requests.length, 1);
  requests[0].resolve({ position: 1 });
  await settle();
  assert.equal(applied.length, 1);
  harness.tick();
  assert.equal(requests.length, 2);
  harness.unmount();
});

test("commands and media changes invalidate stale snapshots without overlapping the old request", async () => {
  const { harness, requests, applied, render } = setup();
  const control = render();
  harness.tick();
  control.invalidatePendingSnapshots();
  requests[0].resolve({ position: 1 });
  await settle();
  assert.equal(applied.length, 0);
  harness.tick();
  render("second");
  harness.tick();
  assert.equal(requests.length, 2);
  requests[1].resolve({ position: 2 });
  await settle();
  assert.equal(applied.length, 0);
  harness.tick();
  requests[2].resolve({ position: 3 });
  await settle();
  assert.equal(applied[0].position, 3);
  harness.unmount();
});

test("request errors recover and unmounted hooks never apply late results", async () => {
  const { harness, requests, applied, render } = setup();
  render();
  harness.tick();
  requests[0].reject(new Error("busy"));
  await settle();
  harness.tick();
  assert.equal(requests.length, 2);
  harness.unmount();
  requests[1].resolve({ position: 10 });
  await settle();
  assert.equal(applied.length, 0);
});
