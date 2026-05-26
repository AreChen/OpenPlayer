import type { PluginActionDefinition, ThemePluginSummary } from "../types";

export function pluginActionStringArg(action: PluginActionDefinition, key: string) {
  const value = action.args?.[key];
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function pluginActionBooleanArg(action: PluginActionDefinition, key: string) {
  return action.args?.[key] === true;
}

export function pluginSettingStringValue(plugin: ThemePluginSummary, settingId: string | null) {
  if (!settingId) {
    return null;
  }
  const setting = plugin.settings.find((candidate) => candidate.id === settingId);
  return typeof setting?.value === "string" && setting.value.trim() ? setting.value.trim() : null;
}

export function pluginSettingBooleanValue(plugin: ThemePluginSummary, settingId: string | null) {
  if (!settingId) {
    return null;
  }
  const setting = plugin.settings.find((candidate) => candidate.id === settingId);
  return typeof setting?.value === "boolean" ? setting.value : null;
}

export function pluginActionStringArgWithSetting(plugin: ThemePluginSummary, action: PluginActionDefinition, key: string, settingKey: string) {
  const settingValue = pluginSettingStringValue(plugin, pluginActionStringArg(action, settingKey));
  return settingValue ?? pluginActionStringArg(action, key);
}

export function pluginActionBooleanArgWithSetting(plugin: ThemePluginSummary, action: PluginActionDefinition, key: string, settingKey: string) {
  const settingValue = pluginSettingBooleanValue(plugin, pluginActionStringArg(action, settingKey));
  return settingValue ?? pluginActionBooleanArg(action, key);
}

export function pluginActionDirectoryArgWithSetting(plugin: ThemePluginSummary, action: PluginActionDefinition) {
  return pluginSettingStringValue(plugin, pluginActionStringArg(action, "directorySetting"));
}

export function runtimeArgsRecord(args: unknown) {
  return args && typeof args === "object" && !Array.isArray(args) ? (args as Record<string, unknown>) : {};
}

export function runtimeStringArg(args: Record<string, unknown>, key: string) {
  const value = args[key];
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function runtimeBooleanArg(args: Record<string, unknown>, key: string) {
  return args[key] === true;
}

export function runtimeNumberArg(args: Record<string, unknown>, key: string) {
  const value = args[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}
