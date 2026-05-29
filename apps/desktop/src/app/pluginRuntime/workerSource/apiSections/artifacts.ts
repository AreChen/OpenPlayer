export function pluginWorkerArtifactsApiSource() {
  return `artifacts: Object.freeze({
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
    })`;
}
