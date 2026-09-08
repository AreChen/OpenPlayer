import assert from "node:assert/strict";
import test from "node:test";
import { loadSource, settle } from "./sourceHarness.mjs";

test("capture menu entry requires host support and loaded media", () => {
  const { buildContextMenuItems } = loadSource("src/app/contextMenu.ts", {
    "./pluginRuntime": { localizedPluginText: (text) => text },
  });
  const options = {
    t: { contextMenu: { externalCapture: "External Enhancement Mode" } },
    shortcutBindings: {}, pluginContextMenuActions: [], isMediaLoaded: true,
  };
  const entry = (overrides) => buildContextMenuItems({ ...options, ...overrides })
    .find((item) => item.id === "external-capture");
  assert.equal(entry({}), undefined);
  let called = false;
  const onEnterCaptureMode = () => { called = true; };
  assert.equal(entry({ onEnterCaptureMode, isMediaLoaded: false }).disabled, true);
  const enabled = entry({ onEnterCaptureMode });
  assert.equal(enabled.disabled, false);
  enabled.onSelect();
  assert.equal(called, true);
});

test("capture action reports backend failures without reactivating hidden controls", async () => {
  const calls = [];
  const errors = [];
  const { useWindowActions } = loadSource("src/hooks/useWindowActions.ts", {
    "@tauri-apps/api/core": { invoke: async (...args) => { calls.push(args); throw new Error("hook unavailable"); } },
    "../app/windowControls": { focusOverlayWindow: () => assert.fail("capture must not refocus overlay") },
  });
  useWindowActions({ media: {}, onError: (error) => errors.push(error) }).enterCaptureMode();
  await settle();
  assert.equal(calls[0][0], "window_set_capture_mode");
  assert.equal(calls[0][1].enabled, true);
  assert.equal(errors[0].message, "hook unavailable");
});
