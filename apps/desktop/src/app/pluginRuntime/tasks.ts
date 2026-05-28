export type PluginTaskStatus = "running" | "cancelRequested" | "completed" | "failed" | "cancelled";

export type PluginTaskJsonValue =
  | null
  | boolean
  | number
  | string
  | PluginTaskJsonValue[]
  | { [key: string]: PluginTaskJsonValue };

export type PluginTaskSnapshot = {
  id: string;
  pluginId: string;
  title: string;
  detail: string | null;
  status: PluginTaskStatus;
  progress: number | null;
  cancellable: boolean;
  createdAtMs: number;
  updatedAtMs: number;
  metadata: PluginTaskJsonValue | null;
  result: PluginTaskJsonValue | null;
  error: string | null;
};

export const MAX_PLUGIN_TASKS_PER_PLUGIN = 64;

let nextPluginTaskSequence = 1;
const pluginTasks = new Map<string, Map<string, PluginTaskSnapshot>>();

export function startPluginTask(pluginId: string, input: Record<string, unknown>) {
  const tasks = tasksForPlugin(pluginId);
  trimCompletedPluginTasks(tasks);
  if (tasks.size >= MAX_PLUGIN_TASKS_PER_PLUGIN) {
    throw new Error("plugin has too many active tasks");
  }

  const now = Date.now();
  const id = `task-${now.toString(36)}-${nextPluginTaskSequence++}`;
  const task: PluginTaskSnapshot = {
    id,
    pluginId,
    title: normalizedTaskText(input.title, "tasks.start requires a title", 120),
    detail: normalizedOptionalTaskText(input.detail, 512),
    status: "running",
    progress: normalizedTaskProgress(input.progress),
    cancellable: input.cancellable === true,
    createdAtMs: now,
    updatedAtMs: now,
    metadata: normalizedOptionalTaskJson(input.metadata, "tasks.start metadata must be JSON-compatible"),
    result: null,
    error: null,
  };
  tasks.set(id, task);
  return cloneTaskSnapshot(task);
}

export function updatePluginTask(pluginId: string, taskId: string, patch: Record<string, unknown>) {
  const task = runningPluginTask(pluginId, taskId, "tasks.update");
  if (patch.title !== undefined) {
    task.title = normalizedTaskText(patch.title, "tasks.update title must be a non-empty string", 120);
  }
  if (patch.detail !== undefined) {
    task.detail = normalizedOptionalTaskText(patch.detail, 512);
  }
  if (patch.progress !== undefined) {
    task.progress = normalizedTaskProgress(patch.progress);
  }
  if (patch.cancellable !== undefined) {
    task.cancellable = patch.cancellable === true;
  }
  if (patch.metadata !== undefined) {
    task.metadata = normalizedOptionalTaskJson(patch.metadata, "tasks.update metadata must be JSON-compatible");
  }
  task.updatedAtMs = Date.now();
  return cloneTaskSnapshot(task);
}

export function completePluginTask(pluginId: string, taskId: string, result: unknown) {
  const task = runningPluginTask(pluginId, taskId, "tasks.complete");
  task.status = "completed";
  task.progress = 1;
  task.result = normalizedOptionalTaskJson(result ?? null, "tasks.complete result must be JSON-compatible");
  task.error = null;
  task.updatedAtMs = Date.now();
  return cloneTaskSnapshot(task);
}

export function failPluginTask(pluginId: string, taskId: string, error: unknown) {
  const task = runningPluginTask(pluginId, taskId, "tasks.fail");
  task.status = "failed";
  task.error = normalizedTaskText(error, "tasks.fail requires an error message", 512);
  task.updatedAtMs = Date.now();
  return cloneTaskSnapshot(task);
}

export function requestPluginTaskCancel(pluginId: string, taskId: string) {
  const task = pluginTask(pluginId, taskId);
  if (isTerminalPluginTaskStatus(task.status)) {
    return cloneTaskSnapshot(task);
  }
  if (!task.cancellable) {
    throw new Error("plugin task is not cancellable");
  }
  task.status = "cancelRequested";
  task.updatedAtMs = Date.now();
  return cloneTaskSnapshot(task);
}

export function markPluginTaskCancelled(pluginId: string, taskId: string) {
  const task = runningPluginTask(pluginId, taskId, "tasks.markCancelled");
  task.status = "cancelled";
  task.updatedAtMs = Date.now();
  return cloneTaskSnapshot(task);
}

export function listPluginTasks(pluginId: string) {
  return [...tasksForPlugin(pluginId).values()]
    .sort((left, right) => left.createdAtMs - right.createdAtMs)
    .map(cloneTaskSnapshot);
}

function tasksForPlugin(pluginId: string) {
  const normalizedPluginId = normalizedTaskText(pluginId, "plugin task requires a plugin id", 160);
  let tasks = pluginTasks.get(normalizedPluginId);
  if (!tasks) {
    tasks = new Map();
    pluginTasks.set(normalizedPluginId, tasks);
  }
  return tasks;
}

function pluginTask(pluginId: string, taskId: string) {
  const task = tasksForPlugin(pluginId).get(normalizedTaskText(taskId, "plugin task id is required", 120));
  if (!task) {
    throw new Error("plugin task was not found");
  }
  return task;
}

function runningPluginTask(pluginId: string, taskId: string, command: string) {
  const task = pluginTask(pluginId, taskId);
  if (isTerminalPluginTaskStatus(task.status)) {
    throw new Error(`${command} cannot modify a finished task`);
  }
  return task;
}

function isTerminalPluginTaskStatus(status: PluginTaskStatus) {
  return status === "completed" || status === "failed" || status === "cancelled";
}

function trimCompletedPluginTasks(tasks: Map<string, PluginTaskSnapshot>) {
  if (tasks.size < MAX_PLUGIN_TASKS_PER_PLUGIN) {
    return;
  }
  const terminalTasks = [...tasks.values()]
    .filter((task) => isTerminalPluginTaskStatus(task.status))
    .sort((left, right) => left.updatedAtMs - right.updatedAtMs);
  for (const task of terminalTasks) {
    if (tasks.size < MAX_PLUGIN_TASKS_PER_PLUGIN) {
      return;
    }
    tasks.delete(task.id);
  }
}

function normalizedTaskText(value: unknown, error: string, maxLength: number) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error(error);
  }
  return value.trim().slice(0, maxLength);
}

function normalizedOptionalTaskText(value: unknown, maxLength: number) {
  if (value === undefined || value === null) {
    return null;
  }
  if (typeof value !== "string") {
    throw new Error("plugin task detail must be a string or null");
  }
  const text = value.trim();
  return text ? text.slice(0, maxLength) : null;
}

function normalizedTaskProgress(value: unknown) {
  if (value === undefined || value === null) {
    return null;
  }
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error("plugin task progress must be a finite number from 0 to 1");
  }
  return Math.min(1, Math.max(0, value));
}

function normalizedOptionalTaskJson(value: unknown, error: string): PluginTaskJsonValue | null {
  if (value === undefined) {
    return null;
  }
  if (!isPluginTaskJsonValue(value)) {
    throw new Error(error);
  }
  return cloneJsonValue(value);
}

function isPluginTaskJsonValue(value: unknown, depth = 0): value is PluginTaskJsonValue {
  if (depth > 24) {
    return false;
  }
  if (value === null || typeof value === "boolean" || typeof value === "string") {
    return true;
  }
  if (typeof value === "number") {
    return Number.isFinite(value);
  }
  if (Array.isArray(value)) {
    return value.every((item) => isPluginTaskJsonValue(item, depth + 1));
  }
  if (typeof value === "object") {
    return Object.entries(value).every(([key, item]) => typeof key === "string" && isPluginTaskJsonValue(item, depth + 1));
  }
  return false;
}

function cloneTaskSnapshot(task: PluginTaskSnapshot): PluginTaskSnapshot {
  return {
    ...task,
    metadata: cloneJsonValue(task.metadata),
    result: cloneJsonValue(task.result),
  };
}

function cloneJsonValue<T extends PluginTaskJsonValue | null>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
