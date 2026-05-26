import { runtimeStringArg } from "../../app/pluginRuntime";
import { PLUGIN_RUNTIME_COMMAND_NOT_HANDLED, type PluginRuntimeCommandHandler } from "./types";

export const handlePluginUiRuntimeCommand: PluginRuntimeCommandHandler = async (context, command, record, _permissions, pluginId) => {
  switch (command) {
    case "ui.toast": {
      const message = runtimeStringArg(record, "message");
      if (!message) {
        throw new Error("ui.toast requires a message");
      }
      const icon = runtimeStringArg(record, "icon");
      context.showCaptureFeedback(icon === "camera" || icon === "record" ? icon : "info", message.slice(0, 180));
      return null;
    }
    case "ui.openSettings": {
      const section = runtimeStringArg(record, "section");
      if (section === "plugins" || section === "playback" || section === "shortcuts" || section === "about" || section === "appearance") {
        context.setSettingsSection(section);
      }
      context.setIsSettingsOpen(true);
      context.setContextMenu(null);
      context.setMediaPanelMode(null);
      return null;
    }
    case "ui.openPanel": {
      const panel = runtimeStringArg(record, "panel");
      if (panel === "playlist") {
        context.setMediaPanelMode(null);
        context.setIsPlaylistOpen(true);
        return null;
      }
      if (panel === "tracks" || panel === "speed" || panel === "loop") {
        context.setIsPlaylistOpen(false);
        context.setMediaPanelMode(panel);
        return null;
      }
      throw new Error("ui.openPanel requires panel playlist, tracks, speed, or loop");
    }
    case "ui.openPluginView": {
      const viewId = runtimeStringArg(record, "viewId");
      if (!viewId) {
        throw new Error("ui.openPluginView requires a viewId");
      }
      await context.openPluginView(pluginId, viewId);
      return null;
    }
    case "ui.closePluginView":
      context.closePluginView();
      return null;
    default:
      return PLUGIN_RUNTIME_COMMAND_NOT_HANDLED;
  }
};
