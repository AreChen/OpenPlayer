import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import { loadSource } from "./sourceHarness.mjs";

const { pluginWorkerNativeApiSource } = loadSource("src/app/pluginRuntime/workerSource/apiSections/native.ts");
test("native SDK control methods share one worker/view source and preserve payloads", async () => {
  const calls = [];
  const api = vm.runInNewContext(`({${pluginWorkerNativeApiSource()}})`, {
    requestHost: async (command, args) => { calls.push(JSON.parse(JSON.stringify({ command, args }))); return true; },
  }).native;
  assert(Object.isFrozen(api));
  assert(Object.isFrozen(api.video));
  await api.start("enhance");
  await api.call("enhance", "describe", { quality: 2 }, { timeoutMs: 100 });
  await api.stopAll();
  await api.video.validatePlan({ stages: [] });
  assert.deepEqual(calls, [
    { command: "native.start", args: { moduleId: "enhance" } },
    { command: "native.call", args: { moduleId: "enhance", method: "describe", params: { quality: 2 }, timeoutMs: 100 } },
    { command: "native.stopAll", args: {} },
    { command: "native.video.validatePlan", args: { stages: [] } },
  ]);
});

test("native bridge enforces permission and trusted plugin identity", async () => {
  const calls = [];
  const unhandled = Symbol("unhandled");
  const { handlePluginNativeRuntimeCommand: handle } = loadSource("src/hooks/pluginRuntimeCommands/native.ts", {
    "@tauri-apps/api/core": { invoke: async (command, args) => calls.push(JSON.parse(JSON.stringify({ command, args }))) },
    "../../app/pluginRuntime": { runtimeStringArg: (record, key) => typeof record[key] === "string" ? record[key] : null },
    "./types": { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED: unhandled },
  });
  const params = { moduleId: "enhance", pluginId: "other.plugin", method: "echo", params: { value: 1 } };
  await assert.rejects(handle({}, "native.call", params, new Set(), "owned.plugin"), /native.process/);
  assert.equal(calls.length, 0);
  const permissions = new Set(["native.process"]);
  await handle({}, "native.call", params, permissions, "owned.plugin");
  assert.equal(calls[0].args.pluginId, "owned.plugin");
  assert.equal(calls[0].command, "plugin_native_call");
  await assert.rejects(handle({}, "native.start", {}, permissions, "owned.plugin"), /moduleId/);
  await assert.rejects(handle({}, "native.unknown", params, permissions, "owned.plugin"), /unsupported/);
  assert.equal(await handle({}, "player.play", {}, permissions, "owned.plugin"), unhandled);
});
