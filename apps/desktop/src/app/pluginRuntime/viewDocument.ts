import type { ThemePluginSummary, ThemeTokens } from "../types";
import { PLUGIN_VIEW_BRIDGE_ID } from "./constants";

export function buildPluginViewDocument(html: string, plugin: ThemePluginSummary, locale: string, theme: ThemeTokens) {
  const bridge = `
<script id="${PLUGIN_VIEW_BRIDGE_ID}">
(() => {
  "use strict";
  const pluginId = ${JSON.stringify(plugin.id)};
  const pending = new Map();
  let nextRequestId = 1;
  const requestHost = (command, args = {}) => {
    if (typeof command !== "string" || !command.trim()) {
      return Promise.reject(new Error("OpenPlayer plugin command is required"));
    }
    const requestId = nextRequestId++;
    window.parent.postMessage({ type: "openplayer:viewRequest", pluginId, requestId, command, args }, "*");
    return new Promise((resolve, reject) => {
      pending.set(requestId, { resolve, reject });
    });
  };
  window.addEventListener("message", (event) => {
    const message = event.data || {};
    if (message.type !== "openplayer:viewResponse" || message.pluginId !== pluginId) {
      return;
    }
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
  });
  window.openplayer = Object.freeze({
    sdkVersion: "1.0.0",
    locale: ${JSON.stringify(locale)},
    theme: Object.freeze(${JSON.stringify(theme)}),
    request: requestHost,
    storage: Object.freeze({
      get(key) {
        return requestHost("plugin.storage.get", { key });
      },
      list() {
        return requestHost("plugin.storage.list");
      },
      set(key, value) {
        return requestHost("plugin.storage.set", { key, value });
      },
      remove(key) {
        return requestHost("plugin.storage.remove", { key });
      },
    }),
    network: Object.freeze({
      request(args) {
        return requestHost("network.request", args);
      },
    }),
    player: Object.freeze({
      wall: Object.freeze({
        open(tiles) {
          return requestHost("player.wall.open", { tiles });
        },
        layout(tiles) {
          return requestHost("player.wall.layout", { tiles });
        },
        snapshot() {
          return requestHost("player.wall.snapshot");
        },
        setVisible(visible) {
          return requestHost("player.wall.setVisible", { visible });
        },
        close() {
          return requestHost("player.wall.close");
        },
      }),
    }),
    ui: Object.freeze({
      toast(message, options = {}) {
        return requestHost("ui.toast", { ...options, message });
      },
      openSettings(section) {
        return requestHost("ui.openSettings", { section });
      },
      closeView() {
        return requestHost("ui.closePluginView");
      },
    }),
  });
  window.dispatchEvent(new CustomEvent("openplayer:ready", { detail: window.openplayer }));
})();
</script>`;
  const style = `
<style>
:root {
  --op-surface: ${theme.surface};
  --op-panel: ${theme.panel};
  --op-panel-strong: ${theme.panelStrong};
  --op-text: ${theme.text};
  --op-muted: ${theme.muted};
  --op-faint: ${theme.faint};
  --op-accent: ${theme.accent};
  --op-danger: ${theme.danger};
  --op-line: ${theme.line};
  --op-control: ${theme.control};
}
</style>`;
  const injection = `${style}\n${bridge}`;
  if (/<\/head>/i.test(html)) {
    return html.replace(/<\/head>/i, `${injection}\n</head>`);
  }
  return `${injection}\n${html}`;
}
