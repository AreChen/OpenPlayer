export function pluginWorkerMessageSource(pluginLabel: string) {
  return `
  globalThis.onmessage = (event) => {
    const message = event.data || {};
    if (message.type === "openplayer:response") {
      const pendingRequest = pending.get(message.requestId);
      if (!pendingRequest) {
        return;
      }
      pending.delete(message.requestId);
      if (message.ok) {
        pendingRequest.resolve(message.result);
      } else {
        pendingRequest.reject(new Error(String(message.error || "OpenPlayer plugin request failed")));
      }
      return;
    }
    if (message.type === "openplayer:hook") {
      const hookId = message.hookId;
      const hook = message.hook;
      Promise.resolve().then(async () => {
        let result = null;
        if (hook === "media.opening") {
          for (const handler of beforeOpenMediaHandlers) {
            const nextResult = await handler(message.payload);
            if (nextResult && typeof nextResult === "object") {
              result = { ...(result || {}), ...nextResult };
            }
          }
        } else if (hook === "plugin.command") {
          const command = message.payload && message.payload.command;
          const handler = typeof command === "string" ? commandHandlers.get(command) : null;
          if (!handler) {
            throw new Error("Plugin command is not registered: " + String(command || ""));
          }
          result = await handler(message.payload && message.payload.args ? message.payload.args : {}, message.payload || {});
        }
        globalThis.postMessage({ type: "openplayer:hookResponse", hookId, ok: true, result });
      }).catch((error) => {
        globalThis.postMessage({
          type: "openplayer:hookResponse",
          hookId,
          ok: false,
          error: String(error && error.message ? error.message : error),
        });
      });
      return;
    }
    if (message.type === "openplayer:event") {
      for (const handler of eventHandlers) {
        try {
          handler(message.event, message.payload);
        } catch (error) {
          globalThis.postMessage({ type: "openplayer:error", message: String(error && error.message ? error.message : error) });
        }
      }
    }
  };
  globalThis.__openplayerPluginReady = async () => {
    for (const handler of readyHandlers) {
      try {
        await handler();
      } catch (error) {
        globalThis.postMessage({ type: "openplayer:error", message: String(error && error.message ? error.message : error) });
      }
    }
  };
  globalThis.postMessage({ type: "openplayer:loaded", label: ${pluginLabel} });
`;
}
