import assert from "node:assert/strict";
import test from "node:test";
import { hookHarness, loadSource, settle } from "./sourceHarness.mjs";

const defaults = { togglePlayback: "Space", seekForward: "ArrowRight" };
const constants = {
  OPENPLAYER_SHORTCUTS_STORAGE_KEY: "shortcuts",
  TEXT_ENTRY_INPUT_TYPES: new Set(),
  defaultShortcutBindings: defaults,
  shortcutActions: Object.keys(defaults),
};
const { pluginShortcutId, pluginShortcutDefinitionsFor, activeShortcutBindings, assignShortcutBinding, readShortcutBindings } = loadSource(
  "src/app/shortcuts.ts",
  { "./constants": constants, "./pluginRuntime": { localizedPluginText: (fallback, labels, locale) => labels?.[locale] ?? fallback } },
  { window: { localStorage: { getItem: () => JSON.stringify({ togglePlayback: "P", "plugin:nr:toggle": "N", unknown: "X" }) } } },
);
const plain = (value) => JSON.parse(JSON.stringify(value));

test("enabled plugin actions have stable, localized, unique shortcut definitions", () => {
  assert.notEqual(pluginShortcutId("a:b", "c"), pluginShortcutId("a", "b:c"));
  const actions = [{ id: "toggle", label: "Toggle", labelI18n: { "zh-CN": "切换" } }];
  const definitions = pluginShortcutDefinitionsFor([
    { id: "nr", name: "NR", enabled: true, actions },
    { id: "fg", name: "FG", enabled: false, actions },
  ], "zh-CN");
  assert.deepEqual(plain(definitions), [{ action: "plugin:nr:toggle", label: "切换", group: "NR" }]);
});

test("persisted plugin shortcuts survive restart without restoring unknown bindings", () => {
  assert.deepEqual(plain(readShortcutBindings()), { togglePlayback: "P", seekForward: "ArrowRight", "plugin:nr:toggle": "N" });
});

test("native bindings include only currently available actions", () => {
  const bindings = { togglePlayback: "Space", "plugin:nr:toggle": "N", "plugin:disabled:toggle": "D" };
  const definitions = [{ action: "togglePlayback" }, { action: "plugin:nr:toggle" }];
  assert.deepEqual(plain(activeShortcutBindings(bindings, definitions)), { togglePlayback: "Space", "plugin:nr:toggle": "N" });
});

test("assigning a shortcut clears conflicts, including saved disabled-plugin shortcuts", () => {
  const bindings = { togglePlayback: "Space", "plugin:nr:toggle": "N", "plugin:disabled:toggle": "D" };
  assert.deepEqual(plain(assignShortcutBinding(bindings, "plugin:nr:toggle", "D")), {
    togglePlayback: "Space", "plugin:nr:toggle": "D", "plugin:disabled:toggle": null,
  });
});

test("plugin shortcut dispatch uses the existing enabled and media checks", () => {
  const { usePluginActions } = loadSource("src/hooks/usePluginActions.ts", {
    "../app/pluginRuntime": {
      isPluginRuntimeActionCommand: (command) => command.startsWith("plugin."),
      pluginActionCommandRequiresMedia: () => false,
      pluginActionStringArg: () => null,
    },
    "../app/shortcuts": { pluginShortcutId },
  });
  const calls = [];
  const makePlugin = (enabled) => ({ id: "nr", enabled, actions: [{ id: "toggle", command: "plugin.toggle", requiresMedia: true }] });
  const options = { media: { path: "movie" }, onError: assert.fail, executePluginRuntimeAction: async (action) => calls.push(action) };
  usePluginActions({ ...options, appearanceState: { plugins: [makePlugin(true)] } }).executePluginShortcut("plugin:nr:toggle");
  usePluginActions({ ...options, media: null, appearanceState: { plugins: [makePlugin(true)] } }).executePluginShortcut("plugin:nr:toggle");
  usePluginActions({ ...options, appearanceState: { plugins: [makePlugin(false)] } }).executePluginShortcut("plugin:nr:toggle");
  assert.equal(calls.length, 1);
  assert.equal(calls[0].action.command, "plugin.toggle");
});

test("native shortcut bridge ignores stale events and only registers active bindings", async () => {
  const harness = hookHarness();
  let nativeListener;
  const invocations = [];
  const events = [];
  const { useShortcutBridge } = loadSource("src/hooks/useShortcutBridge.ts", {
    react: harness.react,
    "@tauri-apps/api/core": { invoke: async (name, args) => invocations.push({ name, args: plain(args) }) },
    "@tauri-apps/api/event": { listen: async (_name, callback) => { nativeListener = callback; return () => {}; } },
    "../app/shortcuts": { activeShortcutBindings, isTextEntryShortcutTarget: () => false, keyboardEventToChord: () => null, releaseShortcutFocusTarget: () => {} },
  }, { window: { addEventListener() {}, removeEventListener() {} } });
  const options = {
    contextMenu: null, isSettingsOpen: false, isNetworkStreamDialogOpen: false, recordingShortcutAction: null,
    shortcutBindings: { "plugin:nr:toggle": "N", "plugin:disabled:toggle": "D" },
    shortcutDefinitions: [{ action: "plugin:nr:toggle" }],
    onRecordUserActivity() {}, onRecordShortcutActivity() {}, onPerformShortcutAction: (action) => events.push(action),
    onAssignShortcut() {}, onCancelRecordingShortcut() {}, onCloseContextMenu() {}, onCloseSettings() {}, onCloseNetworkStreamDialog() {},
  };
  harness.render(() => useShortcutBridge(options));
  nativeListener({ payload: "plugin:disabled:toggle" });
  nativeListener({ payload: "plugin:nr:toggle" });
  assert.deepEqual(events, ["plugin:nr:toggle"]);
  assert.deepEqual(invocations.find(({ name }) => name === "window_update_shortcuts").args.bindings, { "plugin:nr:toggle": "N" });
  harness.render(() => useShortcutBridge({ ...options, shortcutDefinitions: [] }));
  nativeListener({ payload: "plugin:nr:toggle" });
  assert.equal(events.length, 1);
  await settle();
  harness.unmount();
});
