import type { MpvLoadOptions } from "../types";
import { supportedPluginLoadOptionKeys } from "./constants";

export function normalizePluginLoadOptions(value: unknown): MpvLoadOptions {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  const options: MpvLoadOptions = {};
  for (const [rawKey, rawValue] of Object.entries(value as Record<string, unknown>)) {
    const key = rawKey.trim().toLowerCase();
    if (!supportedPluginLoadOptionKeys.has(key) || typeof rawValue !== "string") {
      continue;
    }
    const optionValue = rawValue.trim();
    if (!optionValue || optionValue.length > 128 || optionValue.includes(",") || optionValue.includes("=")) {
      continue;
    }
    options[key] = optionValue;
  }
  return options;
}
