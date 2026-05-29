import type { ActivePluginView, ThemePluginSummary } from "../../app/types";
import { runtimeArgsRecord } from "../../app/pluginRuntime";
import { supportedPluginRuntimeEvents } from "./events";
import type { PluginRuntimeCommandHandler } from "./types";

export type PluginViewEventBridgeState = {
  eventSubscriptions: Set<string>;
};

type HandlePluginViewBridgeMessageOptions = {
  event: MessageEvent;
  activePluginView: ActivePluginView | null;
  plugins: ThemePluginSummary[];
  pluginViewFrame: HTMLIFrameElement | null;
  commandHandler: PluginRuntimeCommandHandler;
  eventState: PluginViewEventBridgeState;
  onRuntimeLog?: (pluginId: string, level: "info" | "warning" | "error", message: string) => void;
};

export function handlePluginViewBridgeMessage({
  event,
  activePluginView,
  plugins,
  pluginViewFrame,
  commandHandler,
  eventState,
  onRuntimeLog,
}: HandlePluginViewBridgeMessageOptions) {
  if (
    !activePluginView ||
    !pluginViewFrame?.contentWindow ||
    event.source !== pluginViewFrame.contentWindow
  ) {
    return;
  }

  const record = event.data && typeof event.data === "object" ? (event.data as Record<string, unknown>) : null;
  if (!record || record.type !== "openplayer:viewRequest" || record.pluginId !== activePluginView.pluginId) {
    return;
  }
  const requestId = typeof record.requestId === "number" ? record.requestId : null;
  const command = typeof record.command === "string" ? record.command : "";
  if (requestId === null || !command) {
    return;
  }

  const plugin = plugins.find((candidate) => candidate.id === activePluginView.pluginId && candidate.enabled);
  const permissions = new Set(plugin?.permissions ?? []);
  try {
    const subscriptionResult = handlePluginViewEventSubscriptionCommand(
      eventState,
      command,
      record.args,
      plugin?.events ?? [],
    );
    if (subscriptionResult.handled) {
      pluginViewFrame.contentWindow?.postMessage(
        {
          type: "openplayer:viewResponse",
          pluginId: activePluginView.pluginId,
          requestId,
          ok: true,
          result: subscriptionResult.result,
        },
        "*",
      );
      return;
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    onRuntimeLog?.(activePluginView.pluginId, "error", `${command}: ${message}`);
    pluginViewFrame.contentWindow?.postMessage(
      {
        type: "openplayer:viewResponse",
        pluginId: activePluginView.pluginId,
        requestId,
        ok: false,
        error: message,
      },
      "*",
    );
    return;
  }

  commandHandler(command, record.args, permissions, activePluginView.pluginId)
    .then((result) => {
      pluginViewFrame.contentWindow?.postMessage(
        {
          type: "openplayer:viewResponse",
          pluginId: activePluginView.pluginId,
          requestId,
          ok: true,
          result: result ?? null,
        },
        "*",
      );
    })
    .catch((error: unknown) => {
      pluginViewFrame.contentWindow?.postMessage(
        {
          type: "openplayer:viewResponse",
          pluginId: activePluginView.pluginId,
          requestId,
          ok: false,
          error: error instanceof Error ? error.message : String(error),
        },
        "*",
      );
    });
}

export function postPluginViewEvent({
  activePluginView,
  pluginViewFrame,
  eventState,
  event,
  payload,
}: {
  activePluginView: ActivePluginView | null;
  pluginViewFrame: HTMLIFrameElement | null;
  eventState: PluginViewEventBridgeState;
  event: string;
  payload: unknown;
}) {
  if (
    !activePluginView ||
    !pluginViewFrame?.contentWindow ||
    (event !== "app.ready" && !eventState.eventSubscriptions.has(event))
  ) {
    return;
  }
  pluginViewFrame.contentWindow.postMessage(
    {
      type: "openplayer:viewEvent",
      pluginId: activePluginView.pluginId,
      event,
      payload,
    },
    "*",
  );
}

function handlePluginViewEventSubscriptionCommand(
  eventState: PluginViewEventBridgeState,
  command: string,
  args: unknown,
  allowedEvents: string[],
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
  if (event !== "app.ready" && !allowedEvents.includes(event)) {
    throw new Error(`plugin runtime event is not declared in manifest: ${event}`);
  }
  if (command === "events.subscribe") {
    eventState.eventSubscriptions.add(event);
  } else {
    eventState.eventSubscriptions.delete(event);
  }
  return { handled: true, result: Array.from(eventState.eventSubscriptions).sort() };
}
