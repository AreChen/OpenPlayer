import { invoke } from "@tauri-apps/api/core";
import { runtimeNumberArg, runtimeStringArg } from "../../app/pluginRuntime";
import type { MpvCorePropertyValue, MpvSnapshot } from "../../app/types";
import {
  PLUGIN_RUNTIME_COMMAND_NOT_HANDLED,
  type PluginRuntimeCommandHandler,
} from "./types";
import { requireLoadedMedia } from "./player/shared";

export const handlePluginMpvRuntimeCommand: PluginRuntimeCommandHandler = async (
  context,
  command,
  record,
  permissions,
  pluginId,
) => {
  switch (command) {
    case "mpv.getProperty": {
      requireMpvPermission(permissions, "mpv.core");
      requireLoadedMedia(context, command);
      const property = runtimeStringArg(record, "property");
      if (!property) {
        throw new Error("mpv.getProperty requires a property");
      }
      return invoke<MpvCorePropertyValue>("mpv_embed_plugin_get_property", { property });
    }
    case "mpv.setProperty": {
      requireMpvPermission(permissions, "mpv.core");
      requireLoadedMedia(context, command);
      const property = runtimeStringArg(record, "property");
      if (!property) {
        throw new Error("mpv.setProperty requires a property");
      }
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_set_property", {
        property,
        value: record.value,
      });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "mpv.command": {
      const mpvCommand = runtimeStringArg(record, "command");
      if (!mpvCommand) {
        throw new Error("mpv.command requires a command");
      }
      requireMpvPermission(permissions, requiredPermissionForMpvCommand(mpvCommand));
      return runMpvCommand(context, mpvCommand, record.args);
    }
    case "mpv.showText": {
      requireMpvPermission(permissions, "mpv.osd");
      const text = runtimeStringArg(record, "text");
      if (!text) {
        throw new Error("mpv.showText requires text");
      }
      const durationMs = runtimeNumberArg(record, "durationMs");
      return runMpvCommand(context, "show-text", durationMs === null ? [text] : [text, durationMs]);
    }
    case "mpv.scriptMessage": {
      requireMpvPermission(permissions, "mpv.scriptMessage");
      return runMpvCommand(context, "script-message", runtimeUnknownArrayArg(record, "args"));
    }
    case "mpv.filters.add": {
      requireMpvPermission(permissions, "mpv.filters");
      requireLoadedMedia(context, command);
      const filterId = runtimeStringArg(record, "filterId");
      const filter = runtimeStringArg(record, "filter");
      if (!filterId || !filter) {
        throw new Error("mpv.filters.add requires filterId and filter");
      }
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_add_video_filter", {
        pluginId,
        filterId,
        filter,
        params: runtimeRecordOrNull(record.params),
      });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "mpv.filters.remove": {
      requireMpvPermission(permissions, "mpv.filters");
      requireLoadedMedia(context, command);
      const filterId = runtimeStringArg(record, "filterId");
      if (!filterId) {
        throw new Error("mpv.filters.remove requires filterId");
      }
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_remove_video_filter", {
        pluginId,
        filterId,
      });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "mpv.audioFilters.add": {
      requireMpvPermission(permissions, "mpv.filters");
      requireLoadedMedia(context, command);
      const filterId = runtimeStringArg(record, "filterId");
      const filter = runtimeStringArg(record, "filter");
      if (!filterId || !filter) {
        throw new Error("mpv.audioFilters.add requires filterId and filter");
      }
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_add_audio_filter", {
        pluginId,
        filterId,
        filter,
        params: runtimeRecordOrNull(record.params),
      });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "mpv.audioFilters.remove": {
      requireMpvPermission(permissions, "mpv.filters");
      requireLoadedMedia(context, command);
      const filterId = runtimeStringArg(record, "filterId");
      if (!filterId) {
        throw new Error("mpv.audioFilters.remove requires filterId");
      }
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_remove_audio_filter", {
        pluginId,
        filterId,
      });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "mpv.setAbLoop": {
      requireMpvPermission(permissions, "mpv.core");
      requireLoadedMedia(context, command);
      const start = runtimeNumberArg(record, "start");
      const end = runtimeNumberArg(record, "end");
      if (start === null || end === null) {
        throw new Error("mpv.setAbLoop requires start and end");
      }
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_set_ab_loop", { start, end });
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "mpv.clearAbLoop": {
      requireMpvPermission(permissions, "mpv.core");
      requireLoadedMedia(context, command);
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_clear_ab_loop");
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};

function requireMpvPermission(permissions: Set<string>, permission: string) {
  if (!permissions.has(permission)) {
    throw new Error(`plugin runtime command requires ${permission}`);
  }
}

function requiredPermissionForMpvCommand(command: string) {
  if (command === "show-text") {
    return "mpv.osd";
  }
  if (command === "script-message") {
    return "mpv.scriptMessage";
  }
  return "mpv.core";
}

async function runMpvCommand(context: Parameters<PluginRuntimeCommandHandler>[0], command: string, args: unknown) {
  requireLoadedMedia(context, `mpv ${command}`);
  context.invalidatePendingSnapshots();
  const snapshot = await invoke<MpvSnapshot>("mpv_embed_plugin_command", {
    command,
    args: Array.isArray(args) ? args : [],
  });
  context.applyCommandSnapshot(snapshot);
  return snapshot;
}

function runtimeUnknownArrayArg(record: Record<string, unknown>, key: string) {
  const value = record[key];
  return Array.isArray(value) ? value : [];
}

function runtimeRecordOrNull(value: unknown) {
  return value && typeof value === "object" && !Array.isArray(value) ? value : null;
}
