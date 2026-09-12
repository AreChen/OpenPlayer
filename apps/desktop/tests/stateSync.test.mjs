import assert from "node:assert/strict";
import test from "node:test";
import { deferred, hookHarness, loadSource, settle } from "./sourceHarness.mjs";

test("unchanged decode preferences never overwrite a native presenter's effective software mode", async () => {
  const harness = hookHarness();
  const defaults = loadSource("src/app/constants/defaults.ts");
  const core = { invoke: async (_command, { settings }) => ({ ...defaults.DEFAULT_PLAYBACK_SETTINGS, ...settings }) };
  const playback = loadSource("src/app/playback.ts", { "./constants": defaults, "@tauri-apps/api/core": core });
  const { usePlaybackSettingsStore } = loadSource("src/hooks/usePlaybackSettingsStore.ts", {
    react: harness.react, "@tauri-apps/api/core": core,
    "../app/constants": defaults, "../app/playback": playback,
  });
  const mode = { current: "hardware" };
  const writes = [];
  const noop = () => {};
  const store = harness.render(() => usePlaybackSettingsStore({
    previousAudibleVolumeRef: { current: 1 }, hardwareDecodingModeRef: mode,
    setHardwareDecodingModeValue: (value) => writes.push(value), setVolumeLevel: noop,
    setPlaybackSpeedValue: noop, setIsVideoFillEnabled: noop,
    setTimeDisplayModeValue: noop, setLoopModeValue: noop,
  }));
  store.applyPlaybackSettingsFromStore(defaults.DEFAULT_PLAYBACK_SETTINGS);
  mode.current = "software"; // mpv snapshot after attaching a presenter
  for (let i = 0; i < 8; i++) store.applyPlaybackSettingsFromStore(defaults.DEFAULT_PLAYBACK_SETTINGS);
  store.persistPlaybackSettings({ volume: 50 });
  await settle();
  assert.equal(mode.current, "software");
  assert.deepEqual(writes, []);
  store.applyPlaybackSettingsFromStore({ hwdecMode: "software" });
  store.applyPlaybackSettingsFromStore({ hwdecMode: "hardware" });
  assert.equal(mode.current, "hardware");
  assert.deepEqual(writes, ["software", "hardware"]);
  mode.current = "software";
  store.applyPlaybackSettingsFromStore({ hwdecMode: "hardware" });
  assert.equal(mode.current, "software");
});

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
