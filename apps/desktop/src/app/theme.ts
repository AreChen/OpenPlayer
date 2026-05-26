import type { AppearanceState, ThemeStyleProperties } from "./types";

export function colorWithAlpha(color: string, alpha: number) {
  const hex = color.trim().replace(/^#/, "");
  if (![3, 6].includes(hex.length) || !/^[\da-f]+$/i.test(hex)) {
    return color;
  }

  const expanded = hex.length === 3 ? hex.split("").map((part) => part + part).join("") : hex;
  const red = Number.parseInt(expanded.slice(0, 2), 16);
  const green = Number.parseInt(expanded.slice(2, 4), 16);
  const blue = Number.parseInt(expanded.slice(4, 6), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

export function hexColorForPicker(color: string | null | undefined) {
  const value = color?.trim() ?? "";
  return /^#[\da-f]{6}$/i.test(value) ? value : "#caa05d";
}

export function browserLanguages() {
  return navigator.languages?.length ? navigator.languages : [navigator.language || "en-US"];
}

export function activeThemeFromAppearance(appearance: AppearanceState | null) {
  if (!appearance) {
    return null;
  }

  return appearance.themes.find((theme) => theme.id === appearance.activeThemeId && theme.enabled) ?? appearance.themes.find((theme) => theme.enabled) ?? null;
}

export function themeStyleVariables(appearance: AppearanceState | null): ThemeStyleProperties | undefined {
  const theme = activeThemeFromAppearance(appearance);
  if (!theme) {
    return undefined;
  }

  const accent = appearance?.accentOverride ?? theme.tokens.accent;
  return {
    "--surface": theme.tokens.surface,
    "--panel": theme.tokens.panel,
    "--panel-strong": theme.tokens.panelStrong,
    "--text": theme.tokens.text,
    "--muted": theme.tokens.muted,
    "--faint": theme.tokens.faint,
    "--accent": accent,
    "--danger": theme.tokens.danger,
    "--line": theme.tokens.line,
    "--control": theme.tokens.control,
    "--scrollbar-thumb": theme.tokens.scrollbarThumb,
    "--scrollbar-thumb-hover": colorWithAlpha(accent, 0.46),
    "--accent-soft": colorWithAlpha(accent, 0.16),
    "--accent-muted": colorWithAlpha(accent, 0.22),
    "--accent-border": colorWithAlpha(accent, 0.42),
    "--accent-ring": colorWithAlpha(accent, 0.82),
  };
}
