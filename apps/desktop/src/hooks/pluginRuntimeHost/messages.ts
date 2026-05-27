import { runtimeArgsRecord } from "../../app/pluginRuntime";
import type { PluginRuntimeWorkerState } from "../../app/types";
import type { PluginRuntimeCommandHandler } from "./types";

type HandlePluginRuntimeWorkerMessageOptions = {
  workerState: PluginRuntimeWorkerState;
  message: unknown;
  hostState: () => unknown;
  commandHandler: PluginRuntimeCommandHandler;
  onRuntimeLog?: (pluginId: string, level: "info" | "warning" | "error", message: string) => void;
};

export function handlePluginRuntimeWorkerMessage({
  workerState,
  message,
  hostState,
  commandHandler,
  onRuntimeLog,
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
    const message = String(record.message || "OpenPlayer plugin runtime error");
    console.warn(`Plugin runtime error in ${workerState.pluginId}`, message);
    onRuntimeLog?.(workerState.pluginId, "error", message);
    return;
  }
  if (type === "openplayer:hookResponse") {
    handlePluginRuntimeHookResponse(workerState, record);
    return;
  }
  if (type === "openplayer:request") {
    handlePluginRuntimeCommandRequest(workerState, record, commandHandler, onRuntimeLog);
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
  onRuntimeLog?: (pluginId: string, level: "info" | "warning" | "error", message: string) => void,
) {
  const requestId = typeof record.requestId === "number" ? record.requestId : null;
  const command = typeof record.command === "string" ? record.command : "";
  if (requestId === null || !command) {
    return;
  }

  try {
    const subscriptionResult = handlePluginRuntimeEventSubscriptionCommand(workerState, command, record.args);
    if (subscriptionResult.handled) {
      workerState.worker.postMessage({
        type: "openplayer:response",
        requestId,
        ok: true,
        result: subscriptionResult.result,
      });
      return;
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    onRuntimeLog?.(workerState.pluginId, "error", `${command}: ${message}`);
    workerState.worker.postMessage({
      type: "openplayer:response",
      requestId,
      ok: false,
      error: message,
    });
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
      const message = error instanceof Error ? error.message : String(error);
      onRuntimeLog?.(workerState.pluginId, "error", `${command}: ${message}`);
      workerState.worker.postMessage({
        type: "openplayer:response",
        requestId,
        ok: false,
        error: message,
      });
    });
}

function handlePluginRuntimeEventSubscriptionCommand(
  workerState: PluginRuntimeWorkerState,
  command: string,
  args: unknown,
): { handled: boolean; result?: unknown } {
  if (command === "events.list") {
    return { handled: true, result: supportedPluginRuntimeEvents };
  }
  if (command !== "events.subscribe" && command !== "events.unsubscribe") {
    return { handled: false };
  }

  const record = runtimeArgsRecord(args);
  const event = typeof record.event === "string" ? record.event : "";
  if (!supportedPluginRuntimeEvents.includes(event)) {
    throw new Error(`unsupported plugin runtime event: ${event}`);
  }
  if (event !== "app.ready" && !workerState.allowedEvents.has(event)) {
    throw new Error(`plugin runtime event is not declared in manifest: ${event}`);
  }
  if (command === "events.subscribe") {
    workerState.eventSubscriptions.add(event);
  } else {
    workerState.eventSubscriptions.delete(event);
  }
  return { handled: true, result: Array.from(workerState.eventSubscriptions).sort() };
}

const supportedPluginRuntimeEvents = Object.freeze([
  "app.ready",
  "media.opening",
  "media.loaded",
  "playback.snapshot",
  "playback.started",
  "playback.paused",
  "playback.ended",
  "playback.stopped",
  "playback.seeked",
  "playback.volumeChanged",
  "playback.speedChanged",
  "tracks.changed",
  "theme.changed",
  "window.fullscreenChanged",
  "plugin.view.opened",
  "plugin.view.closed",
]);
