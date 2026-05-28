export function pluginWorkerLogApiSource() {
  return `log: Object.freeze({
      info(message) {
        return requestHost("plugin.log.info", { message });
      },
      warn(message) {
        return requestHost("plugin.log.warning", { message });
      },
      error(message) {
        return requestHost("plugin.log.error", { message });
      },
    })`;
}
