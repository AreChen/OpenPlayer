export function pluginWorkerCaptureApiSource() {
  return `capture: Object.freeze({
      screenshot(args = {}) {
        return requestHost("player.captureScreenshot", args);
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
