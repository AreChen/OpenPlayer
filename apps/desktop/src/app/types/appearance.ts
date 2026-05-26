import type { CSSProperties } from "react";
import type { LanguageMode } from "../../i18n";
import type { ThemePluginSummary } from "./plugins";

export type ThemeTokens = {
  surface: string;
  panel: string;
  panelStrong: string;
  text: string;
  muted: string;
  faint: string;
  accent: string;
  danger: string;
  line: string;
  control: string;
  scrollbarThumb: string;
  scrollbarThumbHover: string;
};

export type ThemeCatalogItem = {
  id: string;
  name: string;
  version: string;
  source: "builtIn" | "plugin";
  pluginId: string | null;
  enabled: boolean;
  tokens: ThemeTokens;
};

export type AppearanceState = {
  activeThemeId: string;
  accentOverride: string | null;
  themes: ThemeCatalogItem[];
  plugins: ThemePluginSummary[];
};

export type PlayerPreferences = {
  incognitoMode: boolean;
  quietKeyboardControls: boolean;
  languageMode: LanguageMode;
};

export type ThemeStyleProperties = CSSProperties & Record<`--${string}`, string>;
