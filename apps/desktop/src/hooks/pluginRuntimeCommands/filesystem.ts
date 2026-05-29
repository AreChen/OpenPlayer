import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { playableExtensions, subtitleExtensions } from "../../app/constants";
import { runtimeNumberArg, runtimeStringArg } from "../../app/pluginRuntime";
import type { CurrentSubtitleCue, MpvSnapshot } from "../../app/types";
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

type GeneratedSubtitleReadResult = {
  track: GeneratedSubtitleTrack;
  format: string;
  content: string;
  cues: GeneratedSubtitleCue[] | null;
};

type GeneratedSubtitleCue = {
  start: number;
  end: number;
  text: string;
};

type GeneratedSubtitleDocumentPayload =
  | {
      kind: "content";
      payload: ReturnType<typeof generatedSubtitlePayload>;
    }
  | {
      kind: "cues";
      payload: ReturnType<typeof generatedSubtitleCuesPayload>;
    };

type SubtitleStyleWrite = {
  property: string;
  value: string | number;
};

function subtitleStylePatch(record: Record<string, unknown>, command: string): SubtitleStyleWrite[] {
  const patch: SubtitleStyleWrite[] = [];
  pushSubtitleStyleString(patch, record, "fontFamily", "sub-font", 128, command);
  pushSubtitleStyleNumber(patch, record, "fontSize", "sub-font-size", 1, 128, command);
  pushSubtitleStyleNumber(patch, record, "scale", "sub-scale", 0.1, 5, command);
  pushSubtitleStyleNumber(patch, record, "position", "sub-pos", 0, 100, command);
  pushSubtitleStyleColor(patch, record, "color", "sub-color", command);
  pushSubtitleStyleNumber(patch, record, "spacing", "sub-spacing", -10, 10, command);
  pushSubtitleStyleNumber(patch, record, "outlineSize", "sub-outline-size", 0, 32, command);
  pushSubtitleStyleNumber(patch, record, "shadowOffset", "sub-shadow-offset", 0, 32, command);
  if (patch.length === 0) {
    throw new Error(`${command} requires at least one style field`);
  }
  return patch;
}

function pushSubtitleStyleString(
  patch: SubtitleStyleWrite[],
  record: Record<string, unknown>,
  key: string,
  property: string,
  maxLength: number,
  command: string,
) {
  const value = record[key];
  if (value === undefined || value === null) {
    return;
  }
  if (typeof value !== "string" || !value.trim() || value.length > maxLength) {
    throw new Error(`${command} requires ${key} to be non-empty text`);
  }
  patch.push({ property, value });
}

function pushSubtitleStyleNumber(
  patch: SubtitleStyleWrite[],
  record: Record<string, unknown>,
  key: string,
  property: string,
  min: number,
  max: number,
  command: string,
) {
  const value = record[key];
  if (value === undefined || value === null) {
    return;
  }
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) {
    throw new Error(`${command} requires ${key} between ${min} and ${max}`);
  }
  patch.push({ property, value });
}

function pushSubtitleStyleColor(
  patch: SubtitleStyleWrite[],
  record: Record<string, unknown>,
  key: string,
  property: string,
  command: string,
) {
  const value = record[key];
  if (value === undefined || value === null) {
    return;
  }
  if (typeof value !== "string" || !/^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i.test(value)) {
    throw new Error(`${command} requires ${key} to be a hex color`);
  }
  patch.push({ property, value });
}

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

function generatedSubtitleAppendCuesPayload(record: Record<string, unknown>, command: string) {
  if (!Array.isArray(record.cues)) {
    throw new Error(`${command} requires subtitle cues`);
  }
  return {
    cues: record.cues as GeneratedSubtitleCue[],
    select: typeof record.select === "boolean" ? record.select : null,
  };
}

function generatedSubtitleDocumentPayload(record: Record<string, unknown>, command: string): GeneratedSubtitleDocumentPayload {
  const hasContent = typeof record.content === "string" && record.content.trim().length > 0;
  const hasCues = Array.isArray(record.cues);
  if (hasContent && hasCues) {
    throw new Error(`${command} accepts either content or cues, not both`);
  }
  if (hasCues) {
    return { kind: "cues", payload: generatedSubtitleCuesPayload(record, command) };
  }
  return { kind: "content", payload: generatedSubtitlePayload(record, command) };
}

function shouldPersistGeneratedSubtitleSelection(record: Record<string, unknown>) {
  return record.select !== false;
}

function persistGeneratedSubtitleSelection(context: PluginRuntimeCommandContext, snapshot: MpvSnapshot) {
  if (!context.media) {
    return;
  }
  const selectedSubtitle = snapshot.tracks.find((track) => track.kind === "sub" && track.selected);
  context.persistMediaPlaybackSettings(context.media.path, { subtitleTrackId: selectedSubtitle?.id ?? null });
}

function requireLoadedMedia(context: PluginRuntimeCommandContext, command: string) {
  if (!context.media) {
    throw new Error(`${command} requires loaded media`);
  }
}

function requireSubtitleWrite(permissions: Set<string>) {
  if (!permissions.has("subtitle.write")) {
    throw new Error("plugin runtime command requires subtitle.write");
  }
}

async function createGeneratedSubtitleDocument(
  context: PluginRuntimeCommandContext,
  pluginId: string,
  document: GeneratedSubtitleDocumentPayload,
  record: Record<string, unknown>,
) {
  context.invalidatePendingSnapshots();
  const result =
    document.kind === "cues"
      ? await invoke<GeneratedSubtitleLoadResult>("mpv_embed_load_generated_subtitle_cues", {
          pluginId,
          ...document.payload,
        })
      : await invoke<GeneratedSubtitleLoadResult>("mpv_embed_load_generated_subtitle", {
          pluginId,
          ...document.payload,
        });
  context.applyCommandSnapshot(result.snapshot);
  if (shouldPersistGeneratedSubtitleSelection(record)) {
    persistGeneratedSubtitleSelection(context, result.snapshot);
  }
  return result;
}

async function listGeneratedSubtitleDocuments(pluginId: string) {
  return await invoke<GeneratedSubtitleTrack[]>("mpv_embed_list_generated_subtitles", { pluginId });
}

async function readGeneratedSubtitleDocument(pluginId: string, trackId: number) {
  return await invoke<GeneratedSubtitleReadResult>("mpv_embed_read_generated_subtitle", {
    pluginId,
    trackId,
  });
}

async function removeGeneratedSubtitleDocument(context: PluginRuntimeCommandContext, pluginId: string, trackId: number) {
  context.invalidatePendingSnapshots();
  const snapshot = await invoke<MpvSnapshot>("mpv_embed_remove_generated_subtitle", { pluginId, trackId });
  context.applyCommandSnapshot(snapshot);
  persistGeneratedSubtitleSelection(context, snapshot);
  return snapshot;
}

async function replaceGeneratedSubtitleDocument(
  context: PluginRuntimeCommandContext,
  pluginId: string,
  trackId: number,
  document: GeneratedSubtitleDocumentPayload,
  record: Record<string, unknown>,
) {
  context.invalidatePendingSnapshots();
  const result =
    document.kind === "cues"
      ? await invoke<GeneratedSubtitleLoadResult>("mpv_embed_replace_generated_subtitle_cues", {
          pluginId,
          trackId,
          ...document.payload,
        })
      : await invoke<GeneratedSubtitleLoadResult>("mpv_embed_replace_generated_subtitle", {
          pluginId,
          trackId,
          ...document.payload,
        });
  context.applyCommandSnapshot(result.snapshot);
  if (shouldPersistGeneratedSubtitleSelection(record)) {
    persistGeneratedSubtitleSelection(context, result.snapshot);
  }
  return result;
}

async function appendGeneratedSubtitleDocumentCues(
  context: PluginRuntimeCommandContext,
  pluginId: string,
  trackId: number,
  payload: ReturnType<typeof generatedSubtitleAppendCuesPayload>,
  record: Record<string, unknown>,
) {
  context.invalidatePendingSnapshots();
  const result = await invoke<GeneratedSubtitleLoadResult>("mpv_embed_append_generated_subtitle_cues", {
    pluginId,
    trackId,
    cues: payload.cues,
    select: payload.select,
  });
  context.applyCommandSnapshot(result.snapshot);
  if (shouldPersistGeneratedSubtitleSelection(record)) {
    persistGeneratedSubtitleSelection(context, result.snapshot);
  }
  return result;
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
    case "subtitle.currentCue": {
      if (!context.media) {
        throw new Error("subtitle.currentCue requires loaded media");
      }
      if (!permissions.has("subtitle.read")) {
        throw new Error("plugin runtime command requires subtitle.read");
      }
      return await invoke<CurrentSubtitleCue | null>("mpv_embed_current_subtitle_cue");
    }
    case "subtitle.setStyle": {
      if (!context.media) {
        throw new Error("subtitle.setStyle requires loaded media");
      }
      if (!permissions.has("mpv.subtitleStyle")) {
        throw new Error("plugin runtime command requires mpv.subtitleStyle");
      }
      const patch = subtitleStylePatch(record, "subtitle.setStyle");
      context.invalidatePendingSnapshots();
      let snapshot: MpvSnapshot | null = null;
      for (const item of patch) {
        snapshot = await invoke<MpvSnapshot>("mpv_embed_set_plugin_property", item);
      }
      if (!snapshot) {
        throw new Error("subtitle.setStyle did not apply a style field");
      }
      context.applyCommandSnapshot(snapshot);
      return snapshot;
    }
    case "subtitle.documents.create": {
      requireLoadedMedia(context, "subtitle.documents.create");
      requireSubtitleWrite(permissions);
      const document = generatedSubtitleDocumentPayload(record, "subtitle.documents.create");
      return await createGeneratedSubtitleDocument(context, pluginId, document, record);
    }
    case "subtitle.documents.list": {
      requireLoadedMedia(context, "subtitle.documents.list");
      requireSubtitleWrite(permissions);
      return await listGeneratedSubtitleDocuments(pluginId);
    }
    case "subtitle.documents.read": {
      requireLoadedMedia(context, "subtitle.documents.read");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.documents.read");
      return await readGeneratedSubtitleDocument(pluginId, trackId);
    }
    case "subtitle.documents.remove": {
      requireLoadedMedia(context, "subtitle.documents.remove");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.documents.remove");
      return await removeGeneratedSubtitleDocument(context, pluginId, trackId);
    }
    case "subtitle.documents.replace": {
      requireLoadedMedia(context, "subtitle.documents.replace");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.documents.replace");
      const document = generatedSubtitleDocumentPayload(record, "subtitle.documents.replace");
      return await replaceGeneratedSubtitleDocument(context, pluginId, trackId, document, record);
    }
    case "subtitle.documents.appendCues": {
      requireLoadedMedia(context, "subtitle.documents.appendCues");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.documents.appendCues");
      const payload = generatedSubtitleAppendCuesPayload(record, "subtitle.documents.appendCues");
      return await appendGeneratedSubtitleDocumentCues(context, pluginId, trackId, payload, record);
    }
    case "subtitle.loadGenerated": {
      requireLoadedMedia(context, "subtitle.loadGenerated");
      requireSubtitleWrite(permissions);
      const document: GeneratedSubtitleDocumentPayload = {
        kind: "content",
        payload: generatedSubtitlePayload(record, "subtitle.loadGenerated"),
      };
      return await createGeneratedSubtitleDocument(context, pluginId, document, record);
    }
    case "subtitle.loadGeneratedCues": {
      requireLoadedMedia(context, "subtitle.loadGeneratedCues");
      requireSubtitleWrite(permissions);
      const document: GeneratedSubtitleDocumentPayload = {
        kind: "cues",
        payload: generatedSubtitleCuesPayload(record, "subtitle.loadGeneratedCues"),
      };
      return await createGeneratedSubtitleDocument(context, pluginId, document, record);
    }
    case "subtitle.listGenerated": {
      requireLoadedMedia(context, "subtitle.listGenerated");
      requireSubtitleWrite(permissions);
      return await listGeneratedSubtitleDocuments(pluginId);
    }
    case "subtitle.readGenerated": {
      requireLoadedMedia(context, "subtitle.readGenerated");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.readGenerated");
      return await readGeneratedSubtitleDocument(pluginId, trackId);
    }
    case "subtitle.removeGenerated": {
      requireLoadedMedia(context, "subtitle.removeGenerated");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.removeGenerated");
      return await removeGeneratedSubtitleDocument(context, pluginId, trackId);
    }
    case "subtitle.replaceGenerated": {
      requireLoadedMedia(context, "subtitle.replaceGenerated");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.replaceGenerated");
      const document: GeneratedSubtitleDocumentPayload = {
        kind: "content",
        payload: generatedSubtitlePayload(record, "subtitle.replaceGenerated"),
      };
      return await replaceGeneratedSubtitleDocument(context, pluginId, trackId, document, record);
    }
    case "subtitle.replaceGeneratedCues": {
      requireLoadedMedia(context, "subtitle.replaceGeneratedCues");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.replaceGeneratedCues");
      const document: GeneratedSubtitleDocumentPayload = {
        kind: "cues",
        payload: generatedSubtitleCuesPayload(record, "subtitle.replaceGeneratedCues"),
      };
      return await replaceGeneratedSubtitleDocument(context, pluginId, trackId, document, record);
    }
    case "subtitle.appendGeneratedCues": {
      requireLoadedMedia(context, "subtitle.appendGeneratedCues");
      requireSubtitleWrite(permissions);
      const trackId = generatedSubtitleTrackId(record, "subtitle.appendGeneratedCues");
      const payload = generatedSubtitleAppendCuesPayload(record, "subtitle.appendGeneratedCues");
      return await appendGeneratedSubtitleDocumentCues(context, pluginId, trackId, payload, record);
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
