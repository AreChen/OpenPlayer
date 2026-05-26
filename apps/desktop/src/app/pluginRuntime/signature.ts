import type { PluginRuntimeSource } from "../types";

export function pluginRuntimeSignature(source: PluginRuntimeSource) {
  return `${source.pluginId}:${source.version}:${source.entry}:${source.script}`;
}
