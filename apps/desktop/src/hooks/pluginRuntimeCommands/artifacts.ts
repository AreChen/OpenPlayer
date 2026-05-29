import { invoke } from "@tauri-apps/api/core";
import { runtimeStringArg } from "../../app/pluginRuntime";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginArtifactsRuntimeCommand: PluginRuntimeCommandHandler = async (
  _context,
  command,
  record,
  permissions,
  pluginId,
) => {
  switch (command) {
    case "plugin.artifacts.list": {
      const kind = runtimeStringArg(record, "kind");
      if (kind) {
        requireArtifactPermission(kind, permissions);
        return await invoke("plugin_artifacts_list", { pluginId, kind });
      }
      return await listAuthorizedArtifacts(pluginId, permissions);
    }
    case "plugin.artifacts.info": {
      const path = runtimeStringArg(record, "path");
      if (!path) {
        throw new Error("plugin.artifacts.info requires a path");
      }
      const artifact = await invoke<PluginArtifactInfo | null>("plugin_artifacts_info", { pluginId, path });
      if (artifact) {
        requireArtifactPermission(artifact.kind, permissions);
      }
      return artifact;
    }
    case "plugin.artifacts.remove": {
      const path = runtimeStringArg(record, "path");
      if (!path) {
        throw new Error("plugin.artifacts.remove requires a path");
      }
      const artifact = await invoke<PluginArtifactInfo | null>("plugin_artifacts_info", { pluginId, path });
      if (!artifact) {
        return { removed: false, bytesFreed: 0 };
      }
      requireArtifactPermission(artifact.kind, permissions);
      return await invoke("plugin_artifacts_remove", { pluginId, path });
    }
    case "plugin.artifacts.clear": {
      const kind = runtimeStringArg(record, "kind");
      if (kind) {
        requireArtifactPermission(kind, permissions);
        return await invoke("plugin_artifacts_clear", { pluginId, kind });
      }
      return await clearAuthorizedArtifacts(pluginId, permissions);
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};

type PluginArtifactKind = "audioClip" | "frameCapture";

type PluginArtifactInfo = {
  kind: PluginArtifactKind;
  path: string;
  fileName: string;
  mimeType: string;
  sizeBytes: number;
  createdAtMs: number | null;
  modifiedAtMs: number | null;
};

type PluginArtifactClearResult = {
  removedCount: number;
  bytesFreed: number;
};

function requireArtifactPermission(kind: string, permissions: Set<string>) {
  if (kind === "audioClip") {
    if (!permissions.has("audio.extract")) {
      throw new Error("plugin artifact command requires audio.extract");
    }
    return;
  }
  if (kind === "frameCapture") {
    if (!permissions.has("mpv.capture")) {
      throw new Error("plugin artifact command requires mpv.capture");
    }
    return;
  }
  throw new Error("plugin artifact kind must be audioClip or frameCapture");
}

async function listAuthorizedArtifacts(pluginId: string, permissions: Set<string>) {
  const kinds = authorizedArtifactKinds(permissions);
  const results = await Promise.all(
    kinds.map((kind) => invoke<PluginArtifactInfo[]>("plugin_artifacts_list", { pluginId, kind })),
  );
  return results.flat().sort((left, right) => {
    const rightTime = right.modifiedAtMs ?? 0;
    const leftTime = left.modifiedAtMs ?? 0;
    return rightTime - leftTime || left.path.localeCompare(right.path);
  });
}

async function clearAuthorizedArtifacts(pluginId: string, permissions: Set<string>) {
  const results = await Promise.all(
    authorizedArtifactKinds(permissions).map((kind) =>
      invoke<PluginArtifactClearResult>("plugin_artifacts_clear", { pluginId, kind }),
    ),
  );
  return results.reduce(
    (total, result) => ({
      removedCount: total.removedCount + result.removedCount,
      bytesFreed: total.bytesFreed + result.bytesFreed,
    }),
    { removedCount: 0, bytesFreed: 0 },
  );
}

function authorizedArtifactKinds(permissions: Set<string>): PluginArtifactKind[] {
  const kinds: PluginArtifactKind[] = [];
  if (permissions.has("audio.extract")) {
    kinds.push("audioClip");
  }
  if (permissions.has("mpv.capture")) {
    kinds.push("frameCapture");
  }
  return kinds;
}
