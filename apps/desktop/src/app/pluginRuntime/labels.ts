import type { AppStrings } from "../../i18n";
import type {
  PluginActionPlacement,
  PluginCapabilityKind,
  PluginSettingDefinition,
  PluginSettingPlacement,
  PluginSettingValue,
  ThemePluginSummary,
} from "../types";

export function normalizePluginSettingValue(setting: PluginSettingDefinition, value: PluginSettingValue): PluginSettingValue | null {
  switch (setting.kind) {
    case "boolean":
      return typeof value === "boolean" ? value : value === "true";
    case "number": {
      const number = typeof value === "number" ? value : Number(value);
      if (!Number.isFinite(number)) {
        return null;
      }
      const min = typeof setting.min === "number" ? setting.min : -Infinity;
      const max = typeof setting.max === "number" ? setting.max : Infinity;
      return Math.min(max, Math.max(min, number));
    }
    case "select": {
      const selected = String(value);
      return setting.options.some((option) => option.value === selected) ? selected : String(setting.defaultValue);
    }
    case "color":
    case "directory":
    case "text":
      return String(value);
    default:
      return null;
  }
}

export function pluginCapabilityLabel(kind: PluginCapabilityKind, t: AppStrings) {
  return t.settings.plugins.capabilityKinds[kind] ?? kind;
}

export function pluginPlacementLabel(placement: PluginSettingPlacement, t: AppStrings) {
  return t.settings.plugins.placements[placement] ?? placement;
}

export function pluginActionPlacementLabel(placement: PluginActionPlacement, t: AppStrings) {
  return t.settings.plugins.actionPlacements[placement] ?? placement;
}

export function localizedPluginText(fallback: string, localized: Record<string, string> | undefined, locale: string) {
  if (!localized) {
    return fallback;
  }
  const language = locale.split("-")[0];
  return localized[locale] ?? localized[language] ?? localized["en-US"] ?? localized.en ?? fallback;
}

export function pluginPackageKindLabel(kind: ThemePluginSummary["packageKind"], t: AppStrings) {
  return t.settings.plugins.packageKinds[kind] ?? kind;
}
