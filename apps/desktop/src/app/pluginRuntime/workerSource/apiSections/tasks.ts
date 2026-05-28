export function pluginWorkerTasksApiSource() {
  return `tasks: Object.freeze({
      start(input) {
        return requestHost("tasks.start", input);
      },
      update(taskId, patch) {
        return requestHost("tasks.update", { ...patch, taskId });
      },
      complete(taskId, result = null) {
        return requestHost("tasks.complete", { taskId, result });
      },
      fail(taskId, error) {
        return requestHost("tasks.fail", { taskId, error });
      },
      cancel(taskId) {
        return requestHost("tasks.cancel", { taskId });
      },
      markCancelled(taskId) {
        return requestHost("tasks.markCancelled", { taskId });
      },
      list() {
        return requestHost("tasks.list");
      },
    })`;
}
