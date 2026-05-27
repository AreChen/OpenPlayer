import {
  buildPluginWorkerSource,
  pluginRuntimeSignature,
} from "../../app/pluginRuntime";
import type {
  PluginRuntimeSource,
  PluginRuntimeWorkerState,
} from "../../app/types";

type PluginRuntimeWorkerMessageHandler = (
  workerState: PluginRuntimeWorkerState,
  message: unknown,
) => void;

type PluginRuntimeLogHandler = (
  pluginId: string,
  level: "info" | "warning" | "error",
  message: string,
) => void;

export function terminatePluginRuntimeWorker(workerState: PluginRuntimeWorkerState) {
  for (const pendingHook of workerState.pendingHooks.values()) {
    window.clearTimeout(pendingHook.timeout);
    pendingHook.reject(new Error("plugin runtime worker stopped"));
  }
  workerState.pendingHooks.clear();
  workerState.worker.terminate();
  URL.revokeObjectURL(workerState.objectUrl);
}

export function terminateAllPluginRuntimeWorkers(
  workers: Map<string, PluginRuntimeWorkerState>,
) {
  for (const workerState of workers.values()) {
    terminatePluginRuntimeWorker(workerState);
  }
  workers.clear();
}

export function reconcilePluginRuntimeWorkers(
  workers: Map<string, PluginRuntimeWorkerState>,
  sources: PluginRuntimeSource[],
  onMessage: PluginRuntimeWorkerMessageHandler,
  onRuntimeLog?: PluginRuntimeLogHandler,
) {
  for (const [pluginId, workerState] of workers) {
    const source = sources.find((item) => item.pluginId === pluginId);
    if (!source || workerState.signature !== pluginRuntimeSignature(source)) {
      terminatePluginRuntimeWorker(workerState);
      workers.delete(pluginId);
    }
  }

  for (const source of sources) {
    if (workers.has(source.pluginId)) {
      continue;
    }
    startPluginRuntimeWorker(workers, source, onMessage, onRuntimeLog);
  }
}

function startPluginRuntimeWorker(
  workers: Map<string, PluginRuntimeWorkerState>,
  source: PluginRuntimeSource,
  onMessage: PluginRuntimeWorkerMessageHandler,
  onRuntimeLog?: PluginRuntimeLogHandler,
) {
  const objectUrl = URL.createObjectURL(
    new Blob([buildPluginWorkerSource(source)], { type: "text/javascript" }),
  );
  const worker = new Worker(objectUrl, { name: `OpenPlayer plugin ${source.pluginId}` });
  const workerState: PluginRuntimeWorkerState = {
    pluginId: source.pluginId,
    signature: pluginRuntimeSignature(source),
    worker,
    objectUrl,
    permissions: new Set(source.permissions),
    allowedEvents: new Set(source.events),
    eventSubscriptions: new Set(source.events),
    pendingHooks: new Map(),
    nextHookId: 1,
  };

  worker.onmessage = (event) => onMessage(workerState, event.data);
  worker.onerror = (event) => {
    console.warn(`Plugin runtime error in ${source.pluginId}`, event.message);
    onRuntimeLog?.(source.pluginId, "error", event.message);
    event.preventDefault();
  };
  workers.set(source.pluginId, workerState);
}
