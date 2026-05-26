export function pluginWorkerStorageApiSource() {
  return `storage: Object.freeze({
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
    })`;
}
