export function pluginWorkerPluginApiSource() {
  return `plugin: Object.freeze({
      getSettings() {
        return requestHost("plugin.getSettings");
      },
    })`;
}
