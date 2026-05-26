export function pluginWorkerPlaylistApiSource() {
  return `playlist: Object.freeze({
      current() {
        return requestHost("playlist.current");
      },
      playIndex(index) {
        return requestHost("playlist.playIndex", { index });
      },
      clear() {
        return requestHost("playlist.clear");
      },
      openMediaFiles() {
        return requestHost("playlist.openMediaFiles");
      },
      appendMediaFiles() {
        return requestHost("playlist.appendMediaFiles");
      },
    })`;
}
