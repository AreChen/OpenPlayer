export function pluginWorkerCommandsApiSource() {
  return `commands: Object.freeze({
      register(command, handler) {
        if (typeof command === "string" && command.startsWith("plugin.") && typeof handler === "function") {
          commandHandlers.set(command, handler);
        }
      },
    })`;
}
