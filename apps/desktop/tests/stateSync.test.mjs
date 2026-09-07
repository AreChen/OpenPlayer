import assert from "node:assert/strict";
import test from "node:test";
import { deferred, hookHarness, loadSource, settle } from "./sourceHarness.mjs";

test("store sync batches reads, deduplicates catalog state and reconciles optimistic settings using latest callbacks", async () => {
  const harness = hookHarness();
  const calls = [];
  const applied = [];
  const { usePersistentStateSync } = loadSource("src/hooks/usePersistentStateSync.ts", {
    react: harness.react,
    "../app/constants": { STORE_SYNC_INTERVAL_MS: 1600 },
    "@tauri-apps/api/core": { invoke(command) { const pending = deferred(); calls.push({ command, ...pending }); return pending.promise; } },
  }, { window: harness.window });
  const keys = ["onAppearanceState", "onPlayerPreferences", "onPlaybackSettings", "onPlaybackHistory", "onNetworkStreamHistory"];
  const callbacks = (version) => Object.fromEntries(keys.map((key) => [key, (value) => applied.push({ key, value, version })]));
  harness.render(() => usePersistentStateSync(callbacks(1)));
  harness.tick();
  harness.tick();
  assert.deepEqual(calls.map((call) => call.command), ["appearance_sync_state", "playback_sync_state"]);
  const appearance = { appearance: { plugins: [] }, preferences: { incognitoMode: false } };
  const playback = { settings: { volume: 50 }, history: [], networkStreams: [] };
  calls[0].resolve(appearance);
  calls[1].resolve(playback);
  await settle();
  assert.equal(applied.length, 5);
  harness.render(() => usePersistentStateSync(callbacks(2)));
  harness.tick();
  calls[2].resolve(appearance);
  calls[3].resolve(playback);
  await settle();
  assert.equal(applied.length, 6);
  assert.equal(applied.at(-1).key, "onPlaybackSettings");
  assert.equal(applied.at(-1).version, 2);
  harness.tick();
  calls[4].resolve(appearance);
  calls[5].resolve({ ...playback, settings: { volume: 60 } });
  await settle();
  assert.equal(applied.length, 7);
  assert.equal(applied.at(-1).version, 2);
  harness.tick();
  harness.unmount();
  calls[6].resolve({ ...appearance, preferences: { incognitoMode: true } });
  calls[7].resolve(playback);
  await settle();
  assert.equal(applied.length, 7);
});
