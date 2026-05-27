export function pluginWorkerMpvApiSource() {
  return `mpv: Object.freeze({
      getProperty(property) {
        return requestHost("mpv.getProperty", { property });
      },
      setProperty(property, value) {
        return requestHost("mpv.setProperty", { property, value });
      },
      command(command, args = []) {
        return requestHost("mpv.command", { command, args });
      },
      showText(text, options = {}) {
        return requestHost("mpv.showText", { ...options, text });
      },
      scriptMessage(...args) {
        return requestHost("mpv.scriptMessage", { args });
      },
      filters: Object.freeze({
        add(filterId, filter, params = {}) {
          return requestHost("mpv.filters.add", { filterId, filter, params });
        },
        remove(filterId) {
          return requestHost("mpv.filters.remove", { filterId });
        },
      }),
      audioFilters: Object.freeze({
        add(filterId, filter, params = {}) {
          return requestHost("mpv.audioFilters.add", { filterId, filter, params });
        },
        remove(filterId) {
          return requestHost("mpv.audioFilters.remove", { filterId });
        },
      }),
      setAbLoop(start, end) {
        return requestHost("mpv.setAbLoop", { start, end });
      },
      clearAbLoop() {
        return requestHost("mpv.clearAbLoop");
      },
    })`;
}
