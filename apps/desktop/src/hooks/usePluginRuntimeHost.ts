import { useEffect, useRef, type RefObject } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  ActivePluginView,
  PluginActionInstance,
  PluginMediaOpenInput,
  PluginMediaOpenResult,
  PluginRuntimeSource,
  PluginRuntimeWorkerState,
  ThemePluginSummary,
} from "../app/types";
import {
  broadcastPluginRuntimeEvent as broadcastPluginRuntimeEventToWorkers,
  executePluginRuntimeActionForWorker,
  runMediaOpeningHooksForWorkers,
} from "./pluginRuntimeHost/hooks";
import { handlePluginRuntimeWorkerMessage } from "./pluginRuntimeHost/messages";
import type { PluginRuntimeCommandHandler } from "./pluginRuntimeHost/types";
import {
  handlePluginViewBridgeMessage,
  postPluginViewEvent,
  type PluginViewEventBridgeState,
} from "./pluginRuntimeHost/viewBridge";
import {
  reconcilePluginRuntimeWorkers,
  terminateAllPluginRuntimeWorkers,
} from "./pluginRuntimeHost/workers";

type UsePluginRuntimeHostOptions = {
  activePluginView: ActivePluginView | null;
  plugins: ThemePluginSummary[];
  runtimeRefreshKey: string;
  commandHandler: PluginRuntimeCommandHandler;
  hostState: () => unknown;
  pluginViewFrameRef: RefObject<HTMLIFrameElement | null>;
  onRuntimeLog?: (pluginId: string, level: "info" | "warning" | "error", message: string) => void;
};

export function usePluginRuntimeHost({ activePluginView, plugins, runtimeRefreshKey, commandHandler, hostState, pluginViewFrameRef, onRuntimeLog }: UsePluginRuntimeHostOptions) {
  const workersRef = useRef<Map<string, PluginRuntimeWorkerState>>(new Map());
  const commandHandlerRef = useRef<PluginRuntimeCommandHandler>(async () => {
    throw new Error("plugin runtime is not ready");
  });
  const activePluginViewRef = useRef<ActivePluginView | null>(null);
  const activePluginViewKeyRef = useRef<string | null>(null);
  const pluginViewEventStateRef = useRef<PluginViewEventBridgeState>({ eventSubscriptions: new Set() });
  const pluginsRef = useRef<ThemePluginSummary[]>([]);
  const hostStateRef = useRef(hostState);
  const onRuntimeLogRef = useRef(onRuntimeLog);

  commandHandlerRef.current = commandHandler;
  activePluginViewRef.current = activePluginView;
  pluginsRef.current = plugins;
  hostStateRef.current = hostState;
  onRuntimeLogRef.current = onRuntimeLog;
  const activePluginViewPlugin = activePluginView ? plugins.find((plugin) => plugin.id === activePluginView.pluginId) : null;
  const activePluginViewKey = activePluginView
    ? `${activePluginView.pluginId}:${activePluginView.viewId}:${(activePluginViewPlugin?.events ?? []).join("|")}`
    : null;
  if (activePluginViewKeyRef.current !== activePluginViewKey) {
    activePluginViewKeyRef.current = activePluginViewKey;
    pluginViewEventStateRef.current.eventSubscriptions.clear();
  }

  function broadcastPluginRuntimeEvent(event: string, payload: unknown) {
    broadcastPluginRuntimeEventToWorkers(workersRef.current.values(), event, payload);
    postPluginViewEvent({
      activePluginView: activePluginViewRef.current,
      pluginViewFrame: pluginViewFrameRef.current,
      eventState: pluginViewEventStateRef.current,
      event,
      payload,
    });
  }

  async function runMediaOpeningHooks(input: PluginMediaOpenInput): Promise<PluginMediaOpenResult> {
    return runMediaOpeningHooksForWorkers(workersRef.current.values(), input);
  }

  async function executePluginRuntimeAction({ plugin, action }: PluginActionInstance) {
    await executePluginRuntimeActionForWorker(workersRef.current, { plugin, action });
  }

  useEffect(() => {
    let disposed = false;
    invoke<PluginRuntimeSource[]>("appearance_plugin_runtime_sources")
      .then((sources) => {
        if (!disposed) {
          reconcilePluginRuntimeWorkers(
            workersRef.current,
            Array.isArray(sources) ? sources : [],
            (workerState, message) => {
              handlePluginRuntimeWorkerMessage({
                workerState,
                message,
                hostState: () => hostStateRef.current(),
                commandHandler: (command, args, permissions, pluginId) =>
                  commandHandlerRef.current(command, args, permissions, pluginId),
                onRuntimeLog: (pluginId, level, message) =>
                  onRuntimeLogRef.current?.(pluginId, level, message),
              });
            },
            (pluginId, level, message) => onRuntimeLogRef.current?.(pluginId, level, message),
          );
        }
      })
      .catch((error: unknown) => {
        console.warn("Failed to load plugin runtime sources", error);
      });

    return () => {
      disposed = true;
    };
  }, [runtimeRefreshKey]);

  useEffect(() => () => terminateAllPluginRuntimeWorkers(workersRef.current), []);

  useEffect(() => {
    function handlePluginViewMessage(event: MessageEvent) {
      handlePluginViewBridgeMessage({
        event,
        activePluginView: activePluginViewRef.current,
        plugins: pluginsRef.current,
        pluginViewFrame: pluginViewFrameRef.current,
        commandHandler: commandHandlerRef.current,
        eventState: pluginViewEventStateRef.current,
        onRuntimeLog: (pluginId, level, message) =>
          onRuntimeLogRef.current?.(pluginId, level, message),
      });
    }

    window.addEventListener("message", handlePluginViewMessage);
    return () => window.removeEventListener("message", handlePluginViewMessage);
  }, []);

  return {
    broadcastPluginRuntimeEvent,
    executePluginRuntimeAction,
    runMediaOpeningHooks,
  };
}
