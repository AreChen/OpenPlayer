export function pluginWorkerNativeApiSource() {
  return `native: Object.freeze({
    video: Object.freeze({
      validatePlan(plan) { return requestHost("native.video.validatePlan", plan); },
      attach(moduleId, options = {}) { return requestHost("native.video.attach", { moduleId, options }); },
      detach(moduleId) { return requestHost("native.video.detach", { moduleId }); },
      status(moduleId) { return requestHost("native.video.status", { moduleId }); },
    }),
    list() { return requestHost("native.list", {}); },
    start(moduleId) { return requestHost("native.start", { moduleId }); },
    call(moduleId, method, params = null, options = {}) {
      return requestHost("native.call", { moduleId, method, params, timeoutMs: options.timeoutMs });
    },
    stop(moduleId) { return requestHost("native.stop", { moduleId }); },
    stopAll() { return requestHost("native.stopAll", {}); },
  })`;
}
