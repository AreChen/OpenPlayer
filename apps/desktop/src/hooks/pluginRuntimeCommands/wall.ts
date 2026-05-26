import { invoke } from "@tauri-apps/api/core";
import { pluginWallLayouts, pluginWallTiles } from "../../app/pluginRuntime";
import type { MpvWallTileSnapshot } from "../../app/types";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginWallRuntimeCommand: PluginRuntimeCommandHandler = async (context, command, record, permissions) => {
  switch (command) {
    case "player.wall.open": {
      if (!permissions.has("mpv.wall")) {
        throw new Error("plugin runtime command requires mpv.wall");
      }
      return await invoke<MpvWallTileSnapshot[]>("mpv_wall_open", { tiles: pluginWallTiles(record.tiles, context.pluginViewFrameRef.current) });
    }
    case "player.wall.layout": {
      if (!permissions.has("mpv.wall")) {
        throw new Error("plugin runtime command requires mpv.wall");
      }
      return await invoke<MpvWallTileSnapshot[]>("mpv_wall_layout", { tiles: pluginWallLayouts(record.tiles, context.pluginViewFrameRef.current) });
    }
    case "player.wall.snapshot": {
      if (!permissions.has("mpv.wall")) {
        throw new Error("plugin runtime command requires mpv.wall");
      }
      return await invoke<MpvWallTileSnapshot[]>("mpv_wall_snapshot");
    }
    case "player.wall.setVisible": {
      if (!permissions.has("mpv.wall")) {
        throw new Error("plugin runtime command requires mpv.wall");
      }
      await invoke("mpv_wall_set_visible", { visible: record.visible !== false });
      return null;
    }
    case "player.wall.close": {
      if (!permissions.has("mpv.wall")) {
        throw new Error("plugin runtime command requires mpv.wall");
      }
      await invoke("mpv_wall_close");
      return null;
    }
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
