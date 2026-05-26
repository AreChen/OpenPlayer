import { invoke } from "@tauri-apps/api/core";
import { runPluginNetworkRequest, runtimeStringArg } from "../../app/pluginRuntime";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginDataRuntimeCommand: PluginRuntimeCommandHandler = async (context, command, record, permissions, pluginId) => {
  switch (command) {
    case "plugin.getSettings": {
      const plugin = context.appearanceState?.plugins.find((candidate) => candidate.id === pluginId);
      return Object.fromEntries((plugin?.settings ?? []).map((setting) => [setting.id, setting.value]));
    }
    case "plugin.storage.get": {
      const key = runtimeStringArg(record, "key");
      if (!key) {
        throw new Error("plugin storage get requires a key");
      }
      return await invoke("appearance_plugin_kv_get", { pluginId, key });
    }
    case "plugin.storage.list":
      return await invoke("appearance_plugin_kv_list", { pluginId });
    case "plugin.storage.set": {
      const key = runtimeStringArg(record, "key");
      if (!key) {
        throw new Error("plugin storage set requires a key");
      }
      await invoke("appearance_plugin_kv_set", { pluginId, key, value: record.value ?? null });
      return null;
    }
    case "plugin.storage.remove": {
      const key = runtimeStringArg(record, "key");
      if (!key) {
        throw new Error("plugin storage remove requires a key");
      }
      return await invoke("appearance_plugin_kv_remove", { pluginId, key });
    }
    case "network.request": {
      if (!permissions.has("network.request")) {
        throw new Error("plugin runtime command requires network.request");
      }
      return await runPluginNetworkRequest(record);
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
