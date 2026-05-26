import { pluginWorkerApiObjectMembersSource } from "./apiSections";

export function pluginWorkerApiSource() {
  return `
  globalThis.openplayer = Object.freeze({
    sdkVersion: "1.0.0",
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
