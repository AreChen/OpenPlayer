import { runtimeArgsRecord } from "../../app/pluginRuntime";
import type { PluginRuntimeWorkerState } from "../../app/types";
import type { PluginRuntimeCommandHandler } from "./types";

type HandlePluginRuntimeWorkerMessageOptions = {
  workerState: PluginRuntimeWorkerState;
  message: unknown;
  hostState: () => unknown;
  commandHandler: PluginRuntimeCommandHandler;
};

export function handlePluginRuntimeWorkerMessage({
  workerState,
  message,
  hostState,
  commandHandler,
}: HandlePluginRuntimeWorkerMessageOptions) {
  const record = runtimeArgsRecord(message);
  const type = record.type;
  if (type === "openplayer:loaded") {
    return;
  }
  if (type === "openplayer:ready") {
    workerState.worker.postMessage({
      type: "openplayer:event",
      event: "app.ready",
      payload: hostState(),
    });
    return;
  }
  if (type === "openplayer:error") {
    console.warn(`Plugin runtime error in ${workerState.pluginId}`, record.message);
    return;
  }
  if (type === "openplayer:hookResponse") {
    handlePluginRuntimeHookResponse(workerState, record);
    return;
  }
  if (type === "openplayer:request") {
    handlePluginRuntimeCommandRequest(workerState, record, commandHandler);
  }
}

function handlePluginRuntimeHookResponse(
  workerState: PluginRuntimeWorkerState,
  record: Record<string, unknown>,
) {
  const hookId = typeof record.hookId === "number" ? record.hookId : null;
  if (hookId === null) {
    return;
  }
  const pendingHook = workerState.pendingHooks.get(hookId);
  if (!pendingHook) {
    return;
  }
  window.clearTimeout(pendingHook.timeout);
  workerState.pendingHooks.delete(hookId);
  if (record.ok === true) {
    pendingHook.resolve(record.result);
  } else {
    pendingHook.reject(new Error(String(record.error || "OpenPlayer plugin hook failed")));
  }
}

function handlePluginRuntimeCommandRequest(
  workerState: PluginRuntimeWorkerState,
  record: Record<string, unknown>,
  commandHandler: PluginRuntimeCommandHandler,
) {
  const requestId = typeof record.requestId === "number" ? record.requestId : null;
  const command = typeof record.command === "string" ? record.command : "";
  if (requestId === null || !command) {
    return;
  }

  commandHandler(command, record.args, workerState.permissions, workerState.pluginId)
    .then((result) => {
      workerState.worker.postMessage({
        type: "openplayer:response",
        requestId,
        ok: true,
        result: result ?? null,
      });
    })
    .catch((error: unknown) => {
      workerState.worker.postMessage({
        type: "openplayer:response",
        requestId,
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      });
    });
}
