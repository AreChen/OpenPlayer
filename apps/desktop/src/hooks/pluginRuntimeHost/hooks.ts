import {
  PLUGIN_COMMAND_HOOK_TIMEOUT_MS,
  PLUGIN_HOOK_TIMEOUT_MS,
} from "../../app/pluginRuntime";
import type {
  PluginActionInstance,
  PluginMediaOpenInput,
  PluginMediaOpenResult,
  PluginRuntimeWorkerState,
} from "../../app/types";
import {
  mediaOpeningHookPayload,
  normalizePluginMediaOpenResult,
} from "./mediaOpen";

export function broadcastPluginRuntimeEvent(
  workers: Iterable<PluginRuntimeWorkerState>,
  event: string,
  payload: unknown,
) {
  for (const workerState of workers) {
    workerState.worker.postMessage({ type: "openplayer:event", event, payload });
  }
}

export function dispatchPluginRuntimeHook(
  workerState: PluginRuntimeWorkerState,
  hook: string,
  payload: unknown,
  timeoutMs = PLUGIN_HOOK_TIMEOUT_MS,
) {
  return new Promise<unknown>((resolve, reject) => {
    const hookId = workerState.nextHookId++;
    const timeout = window.setTimeout(() => {
      workerState.pendingHooks.delete(hookId);
      reject(new Error(`plugin hook timed out: ${hook}`));
    }, timeoutMs);
    workerState.pendingHooks.set(hookId, { resolve, reject, timeout });
    workerState.worker.postMessage({ type: "openplayer:hook", hookId, hook, payload });
  });
}

export async function runMediaOpeningHooksForWorkers(
  workers: Iterable<PluginRuntimeWorkerState>,
  input: PluginMediaOpenInput,
): Promise<PluginMediaOpenResult> {
  const workerStates = Array.from(workers);
  let result: PluginMediaOpenResult = {
    path: input.path,
    name: input.name,
    loadOptions: { ...input.loadOptions },
  };
  const hookPayload = mediaOpeningHookPayload(input);
  broadcastPluginRuntimeEvent(workerStates, "media.opening", hookPayload);
  for (const workerState of workerStates) {
    try {
      const hookResult = await dispatchPluginRuntimeHook(workerState, "media.opening", {
        ...hookPayload,
        path: result.path,
        name: result.name,
        loadOptions: result.loadOptions,
      });
      result = normalizePluginMediaOpenResult(result, hookResult, workerState.permissions);
    } catch (error) {
      console.warn(`Plugin media.opening hook failed in ${workerState.pluginId}`, error);
    }
  }
  return result;
}

export async function executePluginRuntimeActionForWorker(
  workers: Map<string, PluginRuntimeWorkerState>,
  { plugin, action }: PluginActionInstance,
) {
  const workerState = workers.get(plugin.id);
  if (!workerState) {
    throw new Error(`plugin runtime is unavailable: ${plugin.id}`);
  }
  await dispatchPluginRuntimeHook(
    workerState,
    "plugin.command",
    {
      plugin: {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
      },
      action: {
        id: action.id,
        label: action.label,
        placement: action.placement,
      },
      command: action.command,
      args: action.args,
    },
    PLUGIN_COMMAND_HOOK_TIMEOUT_MS,
  );
}
