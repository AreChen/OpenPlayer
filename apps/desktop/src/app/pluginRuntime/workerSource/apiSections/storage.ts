export function pluginWorkerStorageApiSource() {
  return `storage: Object.freeze({
      get(key) {
        return requestHost("plugin.storage.get", { key });
      },
      list(args) {
        return requestHost("plugin.storage.list", args);
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
    })`;
}
