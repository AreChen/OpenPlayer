import type { MpvWallTileLayout, MpvWallTileRequest } from "../types";
import { runtimeArgsRecord, runtimeStringArg } from "./args";
import { MAX_PLUGIN_WALL_TILES } from "./constants";

type PluginWallFrame = {
  rect: DOMRect;
  viewportWidth: number;
  viewportHeight: number;
};

export function pluginWallTiles(value: unknown, frame: HTMLIFrameElement | null): MpvWallTileRequest[] {
  if (!Array.isArray(value)) {
    throw new Error("player.wall.open requires a tiles array");
  }
  const frameRect = pluginWallFrameRect(frame);
  return value.slice(0, MAX_PLUGIN_WALL_TILES).map((item, index) => pluginWallTile(item, index, frameRect));
}

export function pluginWallLayouts(value: unknown, frame: HTMLIFrameElement | null): MpvWallTileLayout[] {
  if (!Array.isArray(value)) {
    throw new Error("player.wall.layout requires a tiles array");
  }
  const frameRect = pluginWallFrameRect(frame);
  return value.slice(0, MAX_PLUGIN_WALL_TILES).map((item, index) => {
    const record = runtimeArgsRecord(item);
    const id = runtimeStringArg(record, "id") ?? `tile-${index + 1}`;
    const x = clampPluginWallNumber(record.x, 0);
    const y = clampPluginWallNumber(record.y, 0);
    const width = clampPluginWallNumber(record.width, 1);
    const height = clampPluginWallNumber(record.height, 1);
    const tile = pluginWallTileFromFrame(id, x, y, width, height, frameRect);
    return {
      id: tile.id,
      x: tile.x,
      y: tile.y,
      width: tile.width,
      height: tile.height,
    };
  });
}

export function pluginWallFrameRect(frame: HTMLIFrameElement | null) {
  if (!frame) {
    throw new Error("plugin view frame is unavailable");
  }
  const rect = frame.getBoundingClientRect();
  const viewportWidth = Math.max(1, window.innerWidth);
  const viewportHeight = Math.max(1, window.innerHeight);
  if (rect.width <= 0 || rect.height <= 0) {
    throw new Error("plugin view frame has no visible area");
  }
  return { rect, viewportWidth, viewportHeight };
}

export function pluginWallTile(value: unknown, index: number, frame: PluginWallFrame): MpvWallTileRequest {
  const record = runtimeArgsRecord(value);
  const url = runtimeStringArg(record, "url");
  if (!url) {
    throw new Error("player.wall tile requires a url");
  }
  const id = runtimeStringArg(record, "id") ?? `tile-${index + 1}`;
  const x = clampPluginWallNumber(record.x, 0);
  const y = clampPluginWallNumber(record.y, 0);
  const width = clampPluginWallNumber(record.width, 1);
  const height = clampPluginWallNumber(record.height, 1);
  const tile = pluginWallTileFromFrame(id, x, y, width, height, frame);
  return {
    id: tile.id,
    url,
    title: runtimeStringArg(record, "title") ?? undefined,
    x: tile.x,
    y: tile.y,
    width: tile.width,
    height: tile.height,
    muted: record.muted !== false,
  };
}

export function pluginWallTileFromFrame(id: string, x: number, y: number, width: number, height: number, frame: PluginWallFrame) {
  return {
    id,
    x: (frame.rect.left + x * frame.rect.width) / frame.viewportWidth,
    y: (frame.rect.top + y * frame.rect.height) / frame.viewportHeight,
    width: (width * frame.rect.width) / frame.viewportWidth,
    height: (height * frame.rect.height) / frame.viewportHeight,
  };
}

export function clampPluginWallNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : fallback;
}
