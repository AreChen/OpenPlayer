import { invoke } from "@tauri-apps/api/core";
import { runPluginNetworkRequest, runtimeNumberArg, runtimeStringArg } from "../../app/pluginRuntime";
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
    case "plugin.storage.info":
      return await invoke("appearance_plugin_kv_info", { pluginId });
    case "plugin.storage.markMigrated":
      return await invoke("appearance_plugin_kv_mark_migrated", {
        pluginId,
        schemaVersion: runtimeNumberArg(record, "schemaVersion") ?? null,
      });
    case "plugin.storage.set": {
      const key = runtimeStringArg(record, "key");
      if (!key) {
        throw new Error("plugin storage set requires a key");
      }
      await invoke("appearance_plugin_kv_set", { pluginId, key, value: record.value ?? null });
      return null;
    }
    case "plugin.storage.update": {
      const patch = runtimeStorageUpdatePatch(record);
      return await invoke("appearance_plugin_kv_update", { pluginId, ...patch });
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
      return await runPluginNetworkRequest(record, pluginId);
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};

function runtimeStorageUpdatePatch(record: Record<string, unknown>) {
  const rawSet = record.set;
  const set = rawSet === undefined ? {} : runtimeStorageSetValues(rawSet);
  const rawRemove = record.remove;
  const remove = rawRemove === undefined ? [] : runtimeStorageRemoveKeys(rawRemove);
  return { set, remove };
}

function runtimeStorageSetValues(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("plugin storage update set requires an object of key/value pairs");
  }
  return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, item ?? null]));
}

function runtimeStorageRemoveKeys(value: unknown) {
  if (!Array.isArray(value)) {
    throw new Error("plugin storage update remove requires an array of keys");
  }
  return value.map((item) => {
    if (typeof item !== "string" || !item.trim()) {
      throw new Error("plugin storage update remove requires string keys");
    }
    return item.trim();
  });
}
