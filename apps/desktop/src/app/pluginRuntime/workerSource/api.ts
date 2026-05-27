import type { PluginRuntimeSource } from "../../types";
import {
  OPENPLAYER_API_COMPATIBILITY,
  OPENPLAYER_HOST_CAPABILITIES,
  OPENPLAYER_HOST_VERSION,
  PLUGIN_SDK_VERSION,
} from "../constants";
import { pluginWorkerApiObjectMembersSource } from "./apiSections";

export function pluginWorkerApiSource(source: PluginRuntimeSource) {
  const hostCapabilities = JSON.stringify(OPENPLAYER_HOST_CAPABILITIES);
  const hostVersion = JSON.stringify(OPENPLAYER_HOST_VERSION);
  const pluginPermissions = JSON.stringify(source.permissions);
  const apiCompatibility = JSON.stringify(OPENPLAYER_API_COMPATIBILITY);
  const subscribedEvents = JSON.stringify(source.events);
  const sdkVersion = JSON.stringify(PLUGIN_SDK_VERSION);
  return `
  const PLUGIN_SDK_VERSION = ${sdkVersion};
  const OPENPLAYER_HOST_VERSION = ${hostVersion};
  const OPENPLAYER_HOST_CAPABILITIES = Object.freeze(${hostCapabilities});
  const OPENPLAYER_PLUGIN_PERMISSIONS = Object.freeze(${pluginPermissions});
  const OPENPLAYER_API_COMPATIBILITY = Object.freeze(${apiCompatibility});
  const OPENPLAYER_PLUGIN_EVENTS = new Set(${subscribedEvents});
  const updateSubscribedPluginEvents = (events) => {
    if (Array.isArray(events)) {
      OPENPLAYER_PLUGIN_EVENTS.clear();
      for (const event of events) {
        if (typeof event === "string") {
          OPENPLAYER_PLUGIN_EVENTS.add(event);
        }
      }
    }
    return [...OPENPLAYER_PLUGIN_EVENTS];
  };
  globalThis.openplayer = Object.freeze({
    sdkVersion: PLUGIN_SDK_VERSION,
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
    onReady(handler) {
      if (typeof handler === "function") {
        readyHandlers.push(handler);
      }
    },
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
        return [...OPENPLAYER_PLUGIN_EVENTS];
      },
      subscribe(event) {
        return requestHost("events.subscribe", { event }).then(updateSubscribedPluginEvents);
      },
      unsubscribe(event) {
        return requestHost("events.unsubscribe", { event }).then(updateSubscribedPluginEvents);
      },
    }),
    onBeforeOpenMedia(handler) {
      if (typeof handler === "function") {
        beforeOpenMediaHandlers.push(handler);
      }
    },
    registerCommand(command, handler) {
      if (typeof command === "string" && command.startsWith("plugin.") && typeof handler === "function") {
        commandHandlers.set(command, handler);
      }
    },
    ${pluginWorkerApiObjectMembersSource()}
  });
`;
}
