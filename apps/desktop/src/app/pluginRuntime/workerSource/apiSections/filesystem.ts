export function pluginWorkerFilesystemApiSource() {
  return `filesystem: Object.freeze({
      pickMedia(options = {}) {
        return requestHost("filesystem.pickMedia", options);
      },
      pickDirectory() {
        return requestHost("filesystem.pickDirectory");
      },
      revealPath(path) {
        return requestHost("filesystem.revealPath", { path });
      },
      openDirectory(path) {
        return requestHost("filesystem.openDirectory", { path });
      },
    })`;
}
