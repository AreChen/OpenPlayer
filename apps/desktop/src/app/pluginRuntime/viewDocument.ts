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
  const OPENPLAYER_VIEW_EVENT_SUBSCRIPTIONS = new Set();
  const eventHandlers = [];
  const pending = new Map();
  let nextRequestId = 1;
  const updateSubscribedPluginEvents = (events) => {
    if (Array.isArray(events)) {
      OPENPLAYER_VIEW_EVENT_SUBSCRIPTIONS.clear();
      for (const event of events) {
        if (typeof event === "string") {
          OPENPLAYER_VIEW_EVENT_SUBSCRIPTIONS.add(event);
        }
      }
    }
    return [...OPENPLAYER_VIEW_EVENT_SUBSCRIPTIONS];
  };
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
    if (message.type === "openplayer:viewEvent" && message.pluginId === pluginId) {
      for (const handler of eventHandlers) {
        try {
          handler(message.event, message.payload);
        } catch (error) {
          requestHost("plugin.log.error", {
            message: String(error && error.message ? error.message : error),
          }).catch(() => {});
        }
      }
      return;
    }
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
    onEvent(handler) {
      if (typeof handler === "function") {
        eventHandlers.push(handler);
      }
    },
    events: Object.freeze({
      list() {
        return requestHost("events.list");
      },
      subscribed() {
        return [...OPENPLAYER_VIEW_EVENT_SUBSCRIPTIONS];
      },
      subscribe(event) {
        return requestHost("events.subscribe", { event }).then(updateSubscribedPluginEvents);
      },
      unsubscribe(event) {
        return requestHost("events.unsubscribe", { event }).then(updateSubscribedPluginEvents);
      },
    }),
    plugin: Object.freeze({
      getSettings() {
        return requestHost("plugin.getSettings");
      },
    }),
    log: Object.freeze({
      info(message) {
        return requestHost("plugin.log.info", { message });
      },
      warn(message) {
        return requestHost("plugin.log.warning", { message });
      },
      error(message) {
        return requestHost("plugin.log.error", { message });
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
    artifacts: Object.freeze({
      list(args = {}) {
        return requestHost("plugin.artifacts.list", args);
      },
      info(path) {
        return requestHost("plugin.artifacts.info", { path });
      },
      remove(path) {
        return requestHost("plugin.artifacts.remove", { path });
      },
      clear(args = {}) {
        return requestHost("plugin.artifacts.clear", args);
      },
    }),
    storage: Object.freeze({
      get(key) {
        return requestHost("plugin.storage.get", { key });
      },
      list() {
        return requestHost("plugin.storage.list");
      },
      info() {
        return requestHost("plugin.storage.info");
      },
      markMigrated(schemaVersion) {
        return requestHost("plugin.storage.markMigrated", { schemaVersion });
      },
      update(patch) {
        return requestHost("plugin.storage.update", patch);
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
      requestJson(args) {
        const input = args && typeof args === "object" ? args : {};
        const headers = Object.assign({}, input.headers || {});
        if (!headers.Accept && !headers.accept) {
          headers.Accept = "application/json";
        }
        const request = Object.assign({}, input, { headers, responseType: "text" });
        if (Object.prototype.hasOwnProperty.call(input, "body") && input.body !== undefined) {
          request.body = JSON.stringify(input.body);
          if (!headers["Content-Type"] && !headers["content-type"] && !input.bodyFile) {
            headers["Content-Type"] = "application/json";
          }
        }
        return requestHost("network.request", request).then((response) => {
          const json = response.text ? JSON.parse(response.text) : null;
          return Object.assign({}, response, { json });
        });
      },
    }),
    media: Object.freeze({
      current() {
        return requestHost("player.currentMedia");
      },
      currentSegment(args = {}) {
        return requestHost("player.currentSegment", args);
      },
      segmentTimeline(args = {}) {
        return requestHost("player.segmentTimeline", args);
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
    capture: Object.freeze({
      screenshot(args = {}) {
        return requestHost("player.captureScreenshot", args);
      },
      frame(args) {
        return requestHost("capture.frame", args);
      },
      startRecording(args = {}) {
        return requestHost("player.startRecording", args);
      },
      stopRecording(args = {}) {
        return requestHost("player.stopRecording", args);
      },
      toggleRecording(args = {}) {
        return requestHost("player.toggleRecording", args);
      },
      recordingState() {
        return requestHost("player.recordingState");
      },
    }),
    subtitle: Object.freeze({
      pickExternal() {
        return requestHost("subtitle.pickExternal");
      },
      currentCue() {
        return requestHost("subtitle.currentCue");
      },
      setStyle(args) {
        return requestHost("subtitle.setStyle", args);
      },
      documents: Object.freeze({
        create(args) {
          return requestHost("subtitle.documents.create", args);
        },
        list() {
          return requestHost("subtitle.documents.list");
        },
        read(trackId) {
          return requestHost("subtitle.documents.read", { trackId });
        },
        remove(trackId) {
          return requestHost("subtitle.documents.remove", { trackId });
        },
        replace(trackId, args) {
          return requestHost("subtitle.documents.replace", { ...args, trackId });
        },
        appendCues(trackId, args) {
          return requestHost("subtitle.documents.appendCues", { ...args, trackId });
        },
      }),
      loadGenerated(args) {
        return requestHost("subtitle.loadGenerated", args);
      },
      loadGeneratedCues(args) {
        return requestHost("subtitle.loadGeneratedCues", args);
      },
      listGenerated() {
        return requestHost("subtitle.listGenerated");
      },
      readGenerated(trackId) {
        return requestHost("subtitle.readGenerated", { trackId });
      },
      removeGenerated(trackId) {
        return requestHost("subtitle.removeGenerated", { trackId });
      },
      replaceGenerated(trackId, args) {
        return requestHost("subtitle.replaceGenerated", { ...args, trackId });
      },
      replaceGeneratedCues(trackId, args) {
        return requestHost("subtitle.replaceGeneratedCues", { ...args, trackId });
      },
      appendGeneratedCues(trackId, args) {
        return requestHost("subtitle.appendGeneratedCues", { ...args, trackId });
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
  --op-radius: 8px;
  --op-radius-panel: 14px;
  --op-focus-ring: color-mix(in srgb, var(--op-accent) 38%, transparent);
  --op-font: Inter, "Segoe UI", system-ui, sans-serif;
}

.op-view,
.op-view * {
  box-sizing: border-box;
}

.op-view {
  width: 100%;
  height: 100%;
  color: var(--op-text);
  font-family: var(--op-font);
}

.op-surface {
  color: var(--op-text);
  background: color-mix(in srgb, var(--op-panel) 86%, transparent);
  border: 1px solid var(--op-line);
  border-radius: var(--op-radius-panel);
}

.op-stack {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 10px;
}

.op-row {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 8px;
}

.op-section {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 8px;
}

.op-toolbar {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.op-spacer {
  flex: 1 1 auto;
  min-width: 8px;
}

.op-button,
.op-icon-button {
  display: inline-flex;
  min-height: 32px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  color: var(--op-text);
  background: var(--op-control);
  border: 1px solid var(--op-line);
  border-radius: var(--op-radius);
  font: inherit;
  font-weight: 650;
  line-height: 1;
  white-space: nowrap;
  cursor: pointer;
}

.op-button {
  padding: 0 12px;
}

.op-button:hover,
.op-icon-button:hover {
  border-color: color-mix(in srgb, var(--op-accent) 58%, var(--op-line));
  background: color-mix(in srgb, var(--op-accent) 18%, var(--op-control));
}

.op-button:focus-visible,
.op-icon-button:focus-visible,
.op-input:focus,
.op-select:focus,
.op-textarea:focus {
  outline: 2px solid var(--op-focus-ring);
  outline-offset: 2px;
}

.op-button--primary {
  color: var(--op-text);
  border-color: color-mix(in srgb, var(--op-accent) 72%, var(--op-line));
  background: color-mix(in srgb, var(--op-accent) 28%, var(--op-control));
}

.op-icon-button {
  width: 32px;
  padding: 0;
  aspect-ratio: 1;
}

.op-input,
.op-select,
.op-textarea {
  width: 100%;
  min-height: 34px;
  min-width: 0;
  color: var(--op-text);
  background: color-mix(in srgb, var(--op-control) 86%, transparent);
  border: 1px solid var(--op-line);
  border-radius: var(--op-radius);
  font: inherit;
}

.op-input,
.op-select {
  padding: 0 10px;
}

.op-textarea {
  min-height: 82px;
  padding: 9px 10px;
  resize: vertical;
}

.op-input::placeholder,
.op-textarea::placeholder {
  color: var(--op-muted);
}

.op-field {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 6px;
}

.op-label {
  color: var(--op-muted);
  font-size: 12px;
  font-weight: 650;
}

.op-help {
  color: var(--op-muted);
  font-size: 12px;
  line-height: 1.4;
}

.op-divider {
  height: 1px;
  min-height: 1px;
  background: var(--op-line);
  border: 0;
}

.op-tabs {
  display: inline-flex;
  min-width: 0;
  gap: 4px;
  padding: 3px;
  background: color-mix(in srgb, var(--op-control) 78%, transparent);
  border: 1px solid var(--op-line);
  border-radius: var(--op-radius);
}

.op-tab {
  display: inline-flex;
  min-height: 28px;
  align-items: center;
  justify-content: center;
  padding: 0 10px;
  color: var(--op-muted);
  background: transparent;
  border: 1px solid transparent;
  border-radius: calc(var(--op-radius) - 2px);
  font: inherit;
  font-weight: 650;
  line-height: 1;
  white-space: nowrap;
  cursor: pointer;
}

.op-tab[aria-selected="true"],
.op-tab.is-active {
  color: var(--op-text);
  background: color-mix(in srgb, var(--op-accent) 20%, var(--op-control));
  border-color: color-mix(in srgb, var(--op-accent) 58%, var(--op-line));
}

.op-tab:focus-visible {
  outline: 2px solid var(--op-focus-ring);
  outline-offset: 2px;
}

.op-progress {
  width: 100%;
  height: 6px;
  overflow: hidden;
  accent-color: var(--op-accent);
  background: color-mix(in srgb, var(--op-control) 84%, transparent);
  border: 0;
  border-radius: 999px;
}

.op-progress::-webkit-progress-bar {
  background: color-mix(in srgb, var(--op-control) 84%, transparent);
}

.op-progress::-webkit-progress-value {
  background: var(--op-accent);
}

.op-list {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 6px;
}

.op-list-item {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  color: var(--op-text);
  background: color-mix(in srgb, var(--op-control) 58%, transparent);
  border: 1px solid transparent;
  border-radius: var(--op-radius);
}

.op-list-item[aria-selected="true"],
.op-list-item.is-active {
  border-color: color-mix(in srgb, var(--op-accent) 72%, var(--op-line));
  background: color-mix(in srgb, var(--op-accent) 20%, var(--op-control));
}

.op-badge {
  display: inline-flex;
  min-width: 22px;
  min-height: 22px;
  align-items: center;
  justify-content: center;
  padding: 0 7px;
  color: var(--op-muted);
  background: color-mix(in srgb, var(--op-surface) 76%, transparent);
  border: 1px solid var(--op-line);
  border-radius: 999px;
  font-size: 12px;
  line-height: 1;
}

.op-muted {
  color: var(--op-muted);
}

.op-empty {
  display: flex;
  min-width: 0;
  min-height: 92px;
  align-items: center;
  justify-content: center;
  padding: 18px;
  color: var(--op-muted);
  line-height: 1.4;
  text-align: center;
  background: color-mix(in srgb, var(--op-control) 42%, transparent);
  border: 1px dashed var(--op-line);
  border-radius: var(--op-radius);
}
</style>`;
  const injection = `${csp}\n${style}\n${bridge}`;
  if (/<\/head>/i.test(html)) {
    return html.replace(/<\/head>/i, `${injection}\n</head>`);
  }
  return `${injection}\n${html}`;
}
