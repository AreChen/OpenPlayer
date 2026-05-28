export function pluginWorkerSubtitleApiSource() {
  return `subtitle: Object.freeze({
      pickExternal() {
        return requestHost("subtitle.pickExternal");
      },
      loadGenerated(args) {
        return requestHost("subtitle.loadGenerated", args);
      },
      listGenerated() {
        return requestHost("subtitle.listGenerated");
      },
      removeGenerated(trackId) {
        return requestHost("subtitle.removeGenerated", { trackId });
      },
      replaceGenerated(trackId, args) {
        return requestHost("subtitle.replaceGenerated", { ...args, trackId });
      },
      setDelay(delay) {
        return requestHost("player.setSubtitleDelay", { delay });
      },
      selectTrack(trackId) {
        return requestHost("player.selectTrack", { kind: "subtitle", trackId });
      },
    })`;
}
