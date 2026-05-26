import { enUS } from "./i18n/en-US";
import { zhCN } from "./i18n/zh-CN";

export type AppLocale = "en-US" | "zh-CN";
export type LanguageMode = "system" | AppLocale;

export const languageModeOptions: Array<{ mode: LanguageMode; label: Record<AppLocale, string> }> = [
  { mode: "system", label: { "en-US": "Auto", "zh-CN": "自动" } },
  { mode: "en-US", label: { "en-US": "English", "zh-CN": "English" } },
  { mode: "zh-CN", label: { "en-US": "简体中文", "zh-CN": "简体中文" } },
];

export function detectSystemLocale(languages: readonly string[] | undefined): AppLocale {
  const candidates = languages && languages.length ? languages : ["en-US"];
  return candidates.some((language) => language.toLowerCase().startsWith("zh")) ? "zh-CN" : "en-US";
}

export function resolveLocale(mode: LanguageMode, languages: readonly string[] | undefined): AppLocale {
  return mode === "system" ? detectSystemLocale(languages) : mode;
}

export const translations = {
  "en-US": enUS,
  "zh-CN": zhCN,
} satisfies Record<AppLocale, typeof enUS>;

export type AppStrings = (typeof translations)[AppLocale];
