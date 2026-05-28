import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { playableExtensions, subtitleExtensions } from "../../app/constants";
import { runtimeNumberArg, runtimeStringArg } from "../../app/pluginRuntime";
import type { MpvSnapshot } from "../../app/types";
import { focusOverlayWindow } from "../../app/windowControls";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandContext, type PluginRuntimeCommandHandler } from "./types";

type GeneratedSubtitleLoadResult = {
  path: string;
  snapshot: MpvSnapshot;
};

type GeneratedSubtitleTrack = {
  id: number;
  title: string | null;
  language: string | null;
  codec: string | null;
  selected: boolean;
  path: string;
};

type GeneratedSubtitleCue = {
  start: number;
  end: number;
  text: string;
};

function generatedSubtitleTrackId(record: Record<string, unknown>, command: string) {
  const trackId = runtimeNumberArg(record, "trackId");
  if (trackId === null || !Number.isInteger(trackId) || trackId <= 0) {
    throw new Error(`${command} requires a positive numeric trackId`);
  }
  return trackId;
}

function generatedSubtitlePayload(record: Record<string, unknown>, command: string) {
  const format = runtimeStringArg(record, "format");
  const content = typeof record.content === "string" ? record.content : "";
  if (!format) {
    throw new Error(`${command} requires a subtitle format`);
  }
  if (!content.trim()) {
    throw new Error(`${command} requires subtitle content`);
  }
  return {
    name: runtimeStringArg(record, "name"),
    format,
    content,
    select: typeof record.select === "boolean" ? record.select : null,
  };
}

function generatedSubtitleCuesPayload(record: Record<string, unknown>, command: string) {
  const format = runtimeStringArg(record, "format");
  if (!format) {
    throw new Error(`${command} requires a subtitle cue format`);
  }
  if (!Array.isArray(record.cues)) {
    throw new Error(`${command} requires subtitle cues`);
  }
  return {
    name: runtimeStringArg(record, "name"),
    format,
    cues: record.cues as GeneratedSubtitleCue[],
    select: typeof record.select === "boolean" ? record.select : null,
  };
}

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

export const handlePluginFilesystemRuntimeCommand: PluginRuntimeCommandHandler = async (context, command, record, permissions, pluginId) => {
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
    case "subtitle.loadGenerated": {
      if (!context.media) {
        throw new Error("subtitle.loadGenerated requires loaded media");
      }
      if (!permissions.has("subtitle.write")) {
        throw new Error("plugin runtime command requires subtitle.write");
      }
      const payload = generatedSubtitlePayload(record, "subtitle.loadGenerated");
      context.invalidatePendingSnapshots();
      const result = await invoke<GeneratedSubtitleLoadResult>("mpv_embed_load_generated_subtitle", {
        pluginId,
        ...payload,
      });
      context.applyCommandSnapshot(result.snapshot);
      if (record.select !== false) {
        const selectedSubtitle = result.snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
        context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
      }
      return result;
    }
    case "subtitle.loadGeneratedCues": {
      if (!context.media) {
        throw new Error("subtitle.loadGeneratedCues requires loaded media");
      }
      if (!permissions.has("subtitle.write")) {
        throw new Error("plugin runtime command requires subtitle.write");
      }
      const payload = generatedSubtitleCuesPayload(record, "subtitle.loadGeneratedCues");
      context.invalidatePendingSnapshots();
      const result = await invoke<GeneratedSubtitleLoadResult>("mpv_embed_load_generated_subtitle_cues", {
        pluginId,
        ...payload,
      });
      context.applyCommandSnapshot(result.snapshot);
      if (record.select !== false) {
        const selectedSubtitle = result.snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
        context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
      }
      return result;
    }
    case "subtitle.listGenerated": {
      if (!context.media) {
        throw new Error("subtitle.listGenerated requires loaded media");
      }
      if (!permissions.has("subtitle.write")) {
        throw new Error("plugin runtime command requires subtitle.write");
      }
      return await invoke<GeneratedSubtitleTrack[]>("mpv_embed_list_generated_subtitles", { pluginId });
    }
    case "subtitle.removeGenerated": {
      if (!context.media) {
        throw new Error("subtitle.removeGenerated requires loaded media");
      }
      if (!permissions.has("subtitle.write")) {
        throw new Error("plugin runtime command requires subtitle.write");
      }
      const trackId = generatedSubtitleTrackId(record, "subtitle.removeGenerated");
      context.invalidatePendingSnapshots();
      const snapshot = await invoke<MpvSnapshot>("mpv_embed_remove_generated_subtitle", { pluginId, trackId });
      context.applyCommandSnapshot(snapshot);
      const selectedSubtitle = snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
      context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
      return snapshot;
    }
    case "subtitle.replaceGenerated": {
      if (!context.media) {
        throw new Error("subtitle.replaceGenerated requires loaded media");
      }
      if (!permissions.has("subtitle.write")) {
        throw new Error("plugin runtime command requires subtitle.write");
      }
      const trackId = generatedSubtitleTrackId(record, "subtitle.replaceGenerated");
      const payload = generatedSubtitlePayload(record, "subtitle.replaceGenerated");
      context.invalidatePendingSnapshots();
      const result = await invoke<GeneratedSubtitleLoadResult>("mpv_embed_replace_generated_subtitle", {
        pluginId,
        trackId,
        ...payload,
      });
      context.applyCommandSnapshot(result.snapshot);
      if (record.select !== false) {
        const selectedSubtitle = result.snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
        context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
      }
      return result;
    }
    case "subtitle.replaceGeneratedCues": {
      if (!context.media) {
        throw new Error("subtitle.replaceGeneratedCues requires loaded media");
      }
      if (!permissions.has("subtitle.write")) {
        throw new Error("plugin runtime command requires subtitle.write");
      }
      const trackId = generatedSubtitleTrackId(record, "subtitle.replaceGeneratedCues");
      const payload = generatedSubtitleCuesPayload(record, "subtitle.replaceGeneratedCues");
      context.invalidatePendingSnapshots();
      const result = await invoke<GeneratedSubtitleLoadResult>("mpv_embed_replace_generated_subtitle_cues", {
        pluginId,
        trackId,
        ...payload,
      });
      context.applyCommandSnapshot(result.snapshot);
      if (record.select !== false) {
        const selectedSubtitle = result.snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
        context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
      }
      return result;
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
