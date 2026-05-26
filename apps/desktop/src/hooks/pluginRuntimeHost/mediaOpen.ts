import { isMediaStreamPath } from "../../app/media";
import { normalizePluginLoadOptions } from "../../app/pluginRuntime";
import type { PluginMediaOpenInput, PluginMediaOpenResult } from "../../app/types";

export function mediaOpeningHookPayload(input: PluginMediaOpenInput) {
  return {
    ...input,
    isStream: isMediaStreamPath(input.path),
  };
}

export function normalizePluginMediaOpenResult(
  base: PluginMediaOpenResult,
  result: unknown,
  permissions: Set<string>,
) {
  if (!result || typeof result !== "object" || Array.isArray(result)) {
    return base;
  }

  const record = result as Record<string, unknown>;
  const candidatePath = typeof record.path === "string" && record.path.trim() ? record.path.trim() : base.path;
  const nextPath = candidatePath === base.path || permissions.has("media.openStream") ? candidatePath : base.path;
  const nextName = typeof record.name === "string" && record.name.trim() ? record.name.trim().slice(0, 256) : base.name;
  const nextLoadOptions = permissions.has("mpv.loadOptions") ? normalizePluginLoadOptions(record.loadOptions) : {};
  return {
    path: nextPath,
    name: nextName,
    loadOptions: {
      ...base.loadOptions,
      ...nextLoadOptions,
    },
  };
}
