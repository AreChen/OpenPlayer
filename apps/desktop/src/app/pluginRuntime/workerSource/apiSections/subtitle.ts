export function pluginWorkerSubtitleApiSource() {
  return `subtitle: Object.freeze({
      pickExternal() {
        return requestHost("subtitle.pickExternal");
      },
      setDelay(delay) {
        return requestHost("player.setSubtitleDelay", { delay });
      },
      selectTrack(trackId) {
        return requestHost("player.selectTrack", { kind: "subtitle", trackId });
      },
    })`;
}
