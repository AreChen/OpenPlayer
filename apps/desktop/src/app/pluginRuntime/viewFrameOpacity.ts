import type { PluginViewDefinition, ThemePluginSummary } from "../types";

const SIDE_PANEL_FRAME_OPACITY_DEFAULT = 0.82;
const SIDE_PANEL_FRAME_OPACITY_MIN = 0.45;
const SIDE_PANEL_FRAME_OPACITY_MAX = 1;

export function resolvePluginViewFrameOpacity(
  plugin: ThemePluginSummary,
  view: PluginViewDefinition,
) {
  if (view.presentation !== "sidePanel" || !view.frameOpacitySetting) {
    return null;
  }

  const setting = plugin.settings.find((candidate) => candidate.id === view.frameOpacitySetting);
  if (!setting || setting.kind !== "number") {
    return SIDE_PANEL_FRAME_OPACITY_DEFAULT;
  }

  const value = typeof setting.value === "number" ? setting.value : Number(setting.defaultValue);
  if (!Number.isFinite(value)) {
    return SIDE_PANEL_FRAME_OPACITY_DEFAULT;
  }

  return Math.min(
    SIDE_PANEL_FRAME_OPACITY_MAX,
    Math.max(SIDE_PANEL_FRAME_OPACITY_MIN, value),
  );
}
