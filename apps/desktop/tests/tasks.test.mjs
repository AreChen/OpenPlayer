import assert from "node:assert/strict";
import test from "node:test";
import { loadSource } from "./sourceHarness.mjs";

function setup() {
  const api = loadSource("src/app/pluginRuntime/tasks.ts");
  const task = api.startPluginTask("test.plugin", { title: "Work", progress: 0.2, metadata: { chunk: 1 } });
  return { api, task, latest: () => JSON.parse(JSON.stringify(api.listPluginTasks("test.plugin")[0])) };
}

test("invalid completion leaves the task running so error handling can mark it failed", () => {
  const { api, task, latest } = setup();
  const before = latest();
  assert.throws(() => api.completePluginTask("test.plugin", task.id, { value: NaN }), /JSON-compatible/);
  assert.deepEqual(latest(), before);
  api.failPluginTask("test.plugin", task.id, "provider error");
  assert.equal(latest().status, "failed");
});

test("invalid update and failure do not partially mutate a task", () => {
  const { api, task, latest } = setup();
  const before = latest();
  assert.throws(() => api.updatePluginTask("test.plugin", task.id, { title: "Changed", progress: NaN }));
  assert.deepEqual(latest(), before);
  assert.throws(() => api.updatePluginTask("test.plugin", task.id, { progress: 0.8, metadata: { invalid: Infinity } }));
  assert.deepEqual(latest(), before);
  assert.throws(() => api.failPluginTask("test.plugin", task.id, ""));
  assert.deepEqual(latest(), before);
});

test("valid task updates, cancellation and result copies retain their contract", () => {
  const { api, task, latest } = setup();
  api.updatePluginTask("test.plugin", task.id, { title: "Updated", progress: 2, cancellable: true });
  assert.equal(latest().progress, 1);
  api.requestPluginTaskCancel("test.plugin", task.id);
  assert.equal(latest().status, "cancelRequested");
  api.markPluginTaskCancelled("test.plugin", task.id);
  assert.equal(latest().status, "cancelled");
  assert.throws(() => api.completePluginTask("test.plugin", task.id, null), /finished/);
  const other = api.startPluginTask("other.plugin", { title: "Other" });
  assert.throws(() => api.updatePluginTask("test.plugin", other.id, { progress: 1 }), /not found/);
  const result = { chunks: [1] };
  const complete = api.completePluginTask("other.plugin", other.id, result);
  result.chunks.push(2);
  complete.result.chunks.push(3);
  assert.equal(JSON.stringify(api.listPluginTasks("other.plugin")[0].result), '{"chunks":[1]}');
});
