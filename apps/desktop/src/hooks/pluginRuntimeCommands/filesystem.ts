import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { playableExtensions, subtitleExtensions } from "../../app/constants";
import { runtimeStringArg } from "../../app/pluginRuntime";
import type { MpvSnapshot } from "../../app/types";
import { focusOverlayWindow } from "../../app/windowControls";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandContext, type PluginRuntimeCommandHandler } from "./types";

export async function pickPluginMediaPaths(context: PluginRuntimeCommandContext, permissions: Set<string>, multiple = true) {
  if (!permissions.has("filesystem.pick")) {
    throw new Error("plugin runtime command requires filesystem.pick");
  }
  if (context.isPickerOpen) {
    throw new Error("file picker is already open");
  }
  context.setIsPickerOpen(true);
  try {
    const selection = await open({
      multiple,
      filters: [{ name: context.t.dialog.mediaFiles, extensions: playableExtensions }],
    });
    return typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
  } finally {
    context.setIsPickerOpen(false);
    focusOverlayWindow();
  }
}

export const handlePluginFilesystemRuntimeCommand: PluginRuntimeCommandHandler = async (context, command, record, permissions) => {
  switch (command) {
    case "filesystem.pickMedia":
      return await pickPluginMediaPaths(context, permissions, record.multiple !== false);
    case "filesystem.pickDirectory": {
      if (!permissions.has("filesystem.pick")) {
        throw new Error("plugin runtime command requires filesystem.pick");
      }
      if (context.isPickerOpen) {
        throw new Error("file picker is already open");
      }
      context.setIsPickerOpen(true);
      try {
        const selection = await open({ directory: true, multiple: false });
        return typeof selection === "string" ? selection : null;
      } finally {
        context.setIsPickerOpen(false);
        focusOverlayWindow();
      }
    }
    case "filesystem.revealPath": {
      if (!permissions.has("filesystem.reveal")) {
        throw new Error("plugin runtime command requires filesystem.reveal");
      }
      const path = runtimeStringArg(record, "path");
      if (!path) {
        throw new Error("filesystem.revealPath requires a path");
      }
      await invoke("window_reveal_path", { path });
      return null;
    }
    case "filesystem.openDirectory": {
      if (!permissions.has("filesystem.reveal")) {
        throw new Error("plugin runtime command requires filesystem.reveal");
      }
      const path = runtimeStringArg(record, "path");
      if (!path) {
        throw new Error("filesystem.openDirectory requires a path");
      }
      await invoke("window_open_directory", { path });
      return null;
    }
    case "subtitle.pickExternal": {
      if (!context.media) {
        throw new Error("subtitle.pickExternal requires loaded media");
      }
      if (!permissions.has("filesystem.pick")) {
        throw new Error("plugin runtime command requires filesystem.pick");
      }
      if (context.isPickerOpen) {
        throw new Error("file picker is already open");
      }
      context.setIsPickerOpen(true);
      try {
        const selection = await open({
          multiple: false,
          filters: [{ name: context.t.dialog.subtitle, extensions: subtitleExtensions }],
        });
        if (typeof selection !== "string") {
          return null;
        }
        context.invalidatePendingSnapshots();
        const snapshot = await invoke<MpvSnapshot>("mpv_embed_add_subtitle", { path: selection });
        context.applyCommandSnapshot(snapshot);
        const selectedSubtitle = snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
        context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
        return snapshot;
      } finally {
        context.setIsPickerOpen(false);
        focusOverlayWindow();
      }
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
