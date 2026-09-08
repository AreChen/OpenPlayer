import { invoke } from "@tauri-apps/api/core";
import { runtimeStringArg } from "../../app/pluginRuntime";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginNativeRuntimeCommand: PluginRuntimeCommandHandler = async (
  _context, command, record, permissions, pluginId,
) => {
  if (!command.startsWith("native.")) return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  if (!permissions.has("native.process")) throw new Error("native commands require native.process");
  if (command === "native.video.validatePlan") return invoke("plugin_native_validate_video_plan", { pluginId, plan: record });
  if (command === "native.list") return invoke("plugin_native_list", { pluginId });
  if (command === "native.stopAll") return invoke("plugin_native_stop", { pluginId, moduleId: null });
  const moduleId = runtimeStringArg(record, "moduleId");
  if (!moduleId) throw new Error("native command requires moduleId");
  switch (command) {
    case "native.start": return invoke("plugin_native_start", { pluginId, moduleId });
    case "native.stop": return invoke("plugin_native_stop", { pluginId, moduleId });
    case "native.call": return invoke("plugin_native_call", {
      pluginId, moduleId, method: runtimeStringArg(record, "method"),
      params: record.params ?? null, timeoutMs: record.timeoutMs ?? null,
    });
    default: throw new Error(`unsupported native command: ${command}`);
  }
};
