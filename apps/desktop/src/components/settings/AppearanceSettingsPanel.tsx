import { accentSwatches } from "../../app/constants";
import { hexColorForPicker } from "../../app/theme";
import type { AppearanceState, PlayerPreferences, ThemeCatalogItem, ThemeStyleProperties } from "../../app/types";
import { languageModeOptions, type AppLocale, type AppStrings, type LanguageMode } from "../../i18n";

type AppearanceSettingsPanelProps = {
  t: AppStrings;
  locale: AppLocale;
  appearanceState: AppearanceState | null;
  activeTheme: ThemeCatalogItem | null;
  playerPreferences: PlayerPreferences;
  onResetAppearance: () => void;
  onSelectTheme: (themeId: string) => void;
  onSetAccentOverride: (accent: string | null) => void;
  onSetLanguageMode: (mode: LanguageMode) => void;
};

export function AppearanceSettingsPanel({
  t,
  locale,
  appearanceState,
  activeTheme,
  playerPreferences,
  onResetAppearance,
  onSelectTheme,
  onSetAccentOverride,
  onSetLanguageMode,
}: AppearanceSettingsPanelProps) {
  return (
    <section className="settings-panel" aria-labelledby="appearance-settings-title">
      <div className="settings-panel-heading">
        <div>
          <h3 id="appearance-settings-title">{t.settings.appearance.title}</h3>
          <span>{activeTheme ? activeTheme.name : t.common.loading}</span>
        </div>
        <button className="settings-reset" type="button" onClick={onResetAppearance}>
          {t.common.restoreDefaults}
        </button>
      </div>

      <div className="theme-grid" aria-label="Theme selection">
        {(appearanceState?.themes ?? []).map((theme) => {
          const selected = appearanceState?.activeThemeId === theme.id;
          const previewStyle = {
            "--theme-surface": theme.tokens.surface,
            "--theme-panel": theme.tokens.panelStrong,
            "--theme-text": theme.tokens.text,
            "--theme-muted": theme.tokens.muted,
            "--theme-accent": appearanceState?.accentOverride ?? theme.tokens.accent,
          } as ThemeStyleProperties;

          return (
            <button
              key={theme.id}
              className={`theme-card ${selected ? "theme-card--active" : ""}`}
              type="button"
              aria-pressed={selected}
              disabled={!theme.enabled}
              onClick={() => onSelectTheme(theme.id)}
            >
              <span className="theme-preview" style={previewStyle}>
                <span />
                <span />
                <span />
              </span>
              <span className="theme-card-meta">
                <strong>{theme.name}</strong>
                <small>{theme.source === "plugin" ? t.settings.appearance.pluginTheme : t.settings.appearance.builtInTheme}</small>
              </span>
            </button>
          );
        })}
      </div>

      <section className="appearance-section" aria-labelledby="accent-settings-title">
        <header>
          <h4 id="accent-settings-title">{t.settings.appearance.accent}</h4>
          <span>{appearanceState?.accentOverride ? t.settings.appearance.custom : t.settings.appearance.followTheme}</span>
        </header>
        <label className="accent-picker">
          <span>
            <strong>{t.settings.appearance.freePick}</strong>
            <small>{hexColorForPicker(appearanceState?.accentOverride ?? activeTheme?.tokens.accent).toUpperCase()}</small>
          </span>
          <span
            className="accent-picker-preview"
            aria-hidden="true"
            style={{ "--picked-accent": hexColorForPicker(appearanceState?.accentOverride ?? activeTheme?.tokens.accent) } as ThemeStyleProperties}
          />
          <input
            type="color"
            aria-label={t.settings.appearance.freePick}
            value={hexColorForPicker(appearanceState?.accentOverride ?? activeTheme?.tokens.accent)}
            onChange={(event) => onSetAccentOverride(event.currentTarget.value)}
          />
        </label>
        <div className="accent-swatches" role="group" aria-label={t.settings.appearance.accent}>
          <button className={!appearanceState?.accentOverride ? "accent-default accent-swatch--active" : "accent-default"} type="button" onClick={() => onSetAccentOverride(null)}>
            {t.settings.appearance.themeDefault}
          </button>
          {accentSwatches.map((accent) => (
            <button
              key={accent}
              className={appearanceState?.accentOverride === accent ? "accent-swatch accent-swatch--active" : "accent-swatch"}
              type="button"
              aria-label={`${t.settings.appearance.accent} ${accent}`}
              style={{ "--swatch": accent } as ThemeStyleProperties}
              onClick={() => onSetAccentOverride(accent)}
            />
          ))}
        </div>
      </section>

      <section className="appearance-section" aria-labelledby="language-settings-title">
        <header>
          <h4 id="language-settings-title">{t.settings.appearance.language}</h4>
          <span>{t.settings.appearance.languageDescription}</span>
        </header>
        <div className="language-options" role="group" aria-label={t.settings.appearance.language}>
          {languageModeOptions.map((option) => (
            <button
              key={option.mode}
              className={playerPreferences.languageMode === option.mode ? "language-option language-option--active" : "language-option"}
              type="button"
              aria-pressed={playerPreferences.languageMode === option.mode}
              onClick={() => onSetLanguageMode(option.mode)}
            >
              {option.label[locale]}
            </button>
          ))}
        </div>
      </section>
    </section>
  );
}
