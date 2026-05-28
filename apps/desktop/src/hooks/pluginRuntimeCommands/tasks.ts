import {
  completePluginTask,
  failPluginTask,
  listPluginTasks,
  markPluginTaskCancelled,
  requestPluginTaskCancel,
  runtimeStringArg,
  startPluginTask,
  updatePluginTask,
} from "../../app/pluginRuntime";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

function taskIdArg(record: Record<string, unknown>, command: string) {
  const taskId = runtimeStringArg(record, "taskId");
  if (!taskId) {
    throw new Error(`${command} requires a taskId`);
  }
  return taskId;
}

export const handlePluginTasksRuntimeCommand: PluginRuntimeCommandHandler = async (_context, command, record, _permissions, pluginId) => {
  switch (command) {
    case "tasks.start":
      return startPluginTask(pluginId, record);
    case "tasks.update":
      return updatePluginTask(pluginId, taskIdArg(record, command), record);
    case "tasks.complete":
      return completePluginTask(pluginId, taskIdArg(record, command), record.result ?? null);
    case "tasks.fail":
      return failPluginTask(pluginId, taskIdArg(record, command), record.error);
    case "tasks.cancel":
      return requestPluginTaskCancel(pluginId, taskIdArg(record, command));
    case "tasks.markCancelled":
      return markPluginTaskCancelled(pluginId, taskIdArg(record, command));
    case "tasks.list":
      return listPluginTasks(pluginId);
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
