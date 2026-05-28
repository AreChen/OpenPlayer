import type { ThemePluginSummary, ThemeTokens } from "../types";
import {
  OPENPLAYER_API_COMPATIBILITY,
  OPENPLAYER_HOST_CAPABILITIES,
  OPENPLAYER_HOST_VERSION,
  PLUGIN_SDK_VERSION,
  PLUGIN_VIEW_BRIDGE_ID,
} from "./constants";

export function buildPluginViewDocument(html: string, plugin: ThemePluginSummary, locale: string, theme: ThemeTokens) {
  const viewCsp = [
    "default-src 'none'",
    "script-src 'unsafe-inline'",
    "style-src 'unsafe-inline'",
    "img-src data: blob: https:",
    "media-src data: blob:",
    "font-src data:",
    "connect-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
  ].join("; ");
  const bridge = `
<script id="${PLUGIN_VIEW_BRIDGE_ID}">
(() => {
  "use strict";
  const pluginId = ${JSON.stringify(plugin.id)};
  const PLUGIN_SDK_VERSION = ${JSON.stringify(PLUGIN_SDK_VERSION)};
  const OPENPLAYER_HOST_VERSION = ${JSON.stringify(OPENPLAYER_HOST_VERSION)};
  const OPENPLAYER_HOST_CAPABILITIES = Object.freeze(${JSON.stringify(OPENPLAYER_HOST_CAPABILITIES)});
  const OPENPLAYER_PLUGIN_PERMISSIONS = Object.freeze(${JSON.stringify(plugin.permissions)});
  const OPENPLAYER_API_COMPATIBILITY = Object.freeze(${JSON.stringify(OPENPLAYER_API_COMPATIBILITY)});
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
    sdkVersion: PLUGIN_SDK_VERSION,
    locale: ${JSON.stringify(locale)},
    theme: Object.freeze(${JSON.stringify(theme)}),
    api: Object.freeze({
      compatibility: Object.freeze(OPENPLAYER_API_COMPATIBILITY),
    }),
    host: Object.freeze({
      name: "OpenPlayer",
      version: OPENPLAYER_HOST_VERSION,
    }),
    capabilities: Object.freeze({
      list() {
        return [...OPENPLAYER_HOST_CAPABILITIES];
      },
      has(capability) {
        return typeof capability === "string" && OPENPLAYER_HOST_CAPABILITIES.includes(capability);
      },
      permissions() {
        return [...OPENPLAYER_PLUGIN_PERMISSIONS];
      },
      hasPermission(permission) {
        return typeof permission === "string" && OPENPLAYER_PLUGIN_PERMISSIONS.includes(permission);
      },
    }),
    request: requestHost,
    plugin: Object.freeze({
      getSettings() {
        return requestHost("plugin.getSettings");
      },
    }),
    tasks: Object.freeze({
      start(input) {
        return requestHost("tasks.start", input);
      },
      update(taskId, patch) {
        return requestHost("tasks.update", { ...patch, taskId });
      },
      complete(taskId, result = null) {
        return requestHost("tasks.complete", { taskId, result });
      },
      fail(taskId, error) {
        return requestHost("tasks.fail", { taskId, error });
      },
      cancel(taskId) {
        return requestHost("tasks.cancel", { taskId });
      },
      markCancelled(taskId) {
        return requestHost("tasks.markCancelled", { taskId });
      },
      list() {
        return requestHost("tasks.list");
      },
    }),
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
    media: Object.freeze({
      current() {
        return requestHost("player.currentMedia");
      },
      currentSegment(args = {}) {
        return requestHost("player.currentSegment", args);
      },
      snapshot() {
        return requestHost("player.snapshot");
      },
    }),
    audio: Object.freeze({
      extractClip(args) {
        return requestHost("audio.extractClip", args);
      },
    }),
    subtitle: Object.freeze({
      pickExternal() {
        return requestHost("subtitle.pickExternal");
      },
      loadGenerated(args) {
        return requestHost("subtitle.loadGenerated", args);
      },
      listGenerated() {
        return requestHost("subtitle.listGenerated");
      },
      removeGenerated(trackId) {
        return requestHost("subtitle.removeGenerated", { trackId });
      },
      replaceGenerated(trackId, args) {
        return requestHost("subtitle.replaceGenerated", { ...args, trackId });
      },
      setDelay(delay) {
        return requestHost("player.setSubtitleDelay", { delay });
      },
      selectTrack(trackId) {
        return requestHost("player.selectTrack", { kind: "subtitle", trackId });
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
    mpv: Object.freeze({
      getProperty(property) {
        return requestHost("mpv.getProperty", { property });
      },
      setProperty(property, value) {
        return requestHost("mpv.setProperty", { property, value });
      },
      command(command, args = []) {
        return requestHost("mpv.command", { command, args });
      },
      showText(text, options = {}) {
        return requestHost("mpv.showText", { ...options, text });
      },
      scriptMessage(...args) {
        return requestHost("mpv.scriptMessage", { args });
      },
      filters: Object.freeze({
        add(filterId, filter, params = {}) {
          return requestHost("mpv.filters.add", { filterId, filter, params });
        },
        remove(filterId) {
          return requestHost("mpv.filters.remove", { filterId });
        },
      }),
      audioFilters: Object.freeze({
        add(filterId, filter, params = {}) {
          return requestHost("mpv.audioFilters.add", { filterId, filter, params });
        },
        remove(filterId) {
          return requestHost("mpv.audioFilters.remove", { filterId });
        },
      }),
      setAbLoop(start, end) {
        return requestHost("mpv.setAbLoop", { start, end });
      },
      clearAbLoop() {
        return requestHost("mpv.clearAbLoop");
      },
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
  const csp = `<meta http-equiv="Content-Security-Policy" content="${viewCsp}">`;
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
  const injection = `${csp}\n${style}\n${bridge}`;
  if (/<\/head>/i.test(html)) {
    return html.replace(/<\/head>/i, `${injection}\n</head>`);
  }
  return `${injection}\n${html}`;
}
