export function pluginWorkerAudioApiSource() {
  return `audio: Object.freeze({
      extractClip(args) {
        return requestHost("audio.extractClip", args);
      },
    })`;
}
