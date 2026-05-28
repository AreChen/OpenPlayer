export function pluginWorkerSubtitleApiSource() {
  return `subtitle: Object.freeze({
      pickExternal() {
        return requestHost("subtitle.pickExternal");
      },
      currentCue() {
        return requestHost("subtitle.currentCue");
      },
      loadGenerated(args) {
        return requestHost("subtitle.loadGenerated", args);
      },
      loadGeneratedCues(args) {
        return requestHost("subtitle.loadGeneratedCues", args);
      },
      listGenerated() {
        return requestHost("subtitle.listGenerated");
      },
      readGenerated(trackId) {
        return requestHost("subtitle.readGenerated", { trackId });
      },
      removeGenerated(trackId) {
        return requestHost("subtitle.removeGenerated", { trackId });
      },
      replaceGenerated(trackId, args) {
        return requestHost("subtitle.replaceGenerated", { ...args, trackId });
      },
      replaceGeneratedCues(trackId, args) {
        return requestHost("subtitle.replaceGeneratedCues", { ...args, trackId });
      },
      appendGeneratedCues(trackId, args) {
        return requestHost("subtitle.appendGeneratedCues", { ...args, trackId });
      },
      setDelay(delay) {
        return requestHost("player.setSubtitleDelay", { delay });
      },
      selectTrack(trackId) {
        return requestHost("player.selectTrack", { kind: "subtitle", trackId });
      },
    })`;
}
