export function pluginWorkerNativeApiSource() {
  return `native: Object.freeze({
    video: Object.freeze({ validatePlan(plan) { return requestHost("native.video.validatePlan", plan); } }),
    list() { return requestHost("native.list", {}); },
    start(moduleId) { return requestHost("native.start", { moduleId }); },
    call(moduleId, method, params = null, options = {}) {
      return requestHost("native.call", { moduleId, method, params, timeoutMs: options.timeoutMs });
    },
    stop(moduleId) { return requestHost("native.stop", { moduleId }); },
    stopAll() { return requestHost("native.stopAll", {}); },
  })`;
}
