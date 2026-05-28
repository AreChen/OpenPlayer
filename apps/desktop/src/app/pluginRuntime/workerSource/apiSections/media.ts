export function pluginWorkerMediaApiSource() {
  return `media: Object.freeze({
      onBeforeOpen(handler) {
        if (typeof handler === "function") {
          beforeOpenMediaHandlers.push(handler);
        }
      },
      openStream(url, options = {}) {
        return requestHost("player.openStream", { ...options, url });
      },
      openStreamDialog() {
        return requestHost("player.openStreamDialog");
      },
      current() {
        return requestHost("player.currentMedia");
      },
      currentSegment(args = {}) {
        return requestHost("player.currentSegment", args);
      },
      segmentTimeline(args = {}) {
        return requestHost("player.segmentTimeline", args);
      },
      snapshot() {
        return requestHost("player.snapshot");
      },
    })`;
}
