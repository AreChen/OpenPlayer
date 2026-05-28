export function pluginWorkerCaptureApiSource() {
  return `capture: Object.freeze({
      screenshot(args = {}) {
        return requestHost("player.captureScreenshot", args);
      },
      frame(args) {
        return requestHost("capture.frame", args);
      },
      startRecording(args = {}) {
        return requestHost("player.startRecording", args);
      },
      stopRecording(args = {}) {
        return requestHost("player.stopRecording", args);
      },
      toggleRecording(args = {}) {
        return requestHost("player.toggleRecording", args);
      },
      recordingState() {
        return requestHost("player.recordingState");
      },
    })`;
}
