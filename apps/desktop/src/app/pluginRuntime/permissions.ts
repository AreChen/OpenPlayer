import type { AppStrings } from "../../i18n";

export type PluginPermissionRisk = "normal" | "warning" | "danger";

export function pluginPermissionRisk(permission: string): PluginPermissionRisk {
  if (
    permission === "mpv.core" ||
    permission === "mpv.scriptMessage" ||
    permission === "network.request" ||
    permission === "filesystem.pick"
  ) {
    return "danger";
  }
  if (
    permission === "mpv.filters" ||
    permission === "mpv.wall" ||
    permission === "filesystem.reveal" ||
    permission === "media.openStream" ||
    permission === "audio.extract" ||
    permission === "subtitle.read" ||
    permission === "subtitle.write"
  ) {
    return "warning";
  }
  return "normal";
}

export function pluginPermissionDescription(permission: string, t: AppStrings) {
  const descriptions = t.settings.plugins.permissionDescriptions as Record<string, string>;
  return descriptions[permission] ?? descriptions.default;
}
