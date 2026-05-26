import type { ActivePluginView, ThemePluginSummary } from "../../app/types";
import type { PluginRuntimeCommandHandler } from "./types";

type HandlePluginViewBridgeMessageOptions = {
  event: MessageEvent;
  activePluginView: ActivePluginView | null;
  plugins: ThemePluginSummary[];
  pluginViewFrame: HTMLIFrameElement | null;
  commandHandler: PluginRuntimeCommandHandler;
};

export function handlePluginViewBridgeMessage({
  event,
  activePluginView,
  plugins,
  pluginViewFrame,
  commandHandler,
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
