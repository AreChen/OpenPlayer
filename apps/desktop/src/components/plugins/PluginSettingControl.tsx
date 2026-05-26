import { localizedPluginText, pluginPlacementLabel } from "../../app/pluginRuntime";
import type { PluginSettingDefinition, PluginSettingValue, ThemePluginSummary } from "../../app/types";
import type { AppStrings } from "../../i18n";

type PluginSettingControlProps = {
  plugin: ThemePluginSummary;
  setting: PluginSettingDefinition;
  compact?: boolean;
  locale: string;
  t: AppStrings;
  isPickerOpen: boolean;
  systemFontFamilies: string[];
  onValueChange: (pluginId: string, setting: PluginSettingDefinition, value: PluginSettingValue) => void;
  onChooseDirectory: (pluginId: string, setting: PluginSettingDefinition) => void;
  onOpenDirectory: (setting: PluginSettingDefinition) => void;
};

export function PluginSettingControl({
  plugin,
  setting,
  compact = false,
  locale,
  t,
  isPickerOpen,
  systemFontFamilies,
  onValueChange,
  onChooseDirectory,
  onOpenDirectory,
}: PluginSettingControlProps) {
  const controlId = `plugin-${plugin.id}-${setting.id}-${compact ? "compact" : "settings"}`;
  const rowClassName = compact ? "plugin-setting plugin-setting--compact" : "plugin-setting";
  const disabled = !plugin.enabled;
  const settingLabel = localizedPluginText(setting.label, setting.labelI18n, locale);
  const settingDescription = localizedPluginText(setting.description ?? pluginPlacementLabel(setting.placement, t), setting.descriptionI18n, locale);

  const settingHeader = (
    <span className="plugin-setting-copy">
      <strong>{settingLabel}</strong>
      <small>
        {settingDescription}
        {setting.mpvProperty ? ` · ${setting.mpvProperty}` : ""}
      </small>
    </span>
  );

  if (setting.kind === "boolean") {
    return (
      <label className={rowClassName} htmlFor={controlId}>
        {settingHeader}
        <input
          id={controlId}
          type="checkbox"
          disabled={disabled}
          checked={setting.value === true}
          onChange={(event) => onValueChange(plugin.id, setting, event.currentTarget.checked)}
        />
        <span className="preference-switch" aria-hidden="true">
          <span />
        </span>
      </label>
    );
  }

  if (setting.kind === "number") {
    const value = typeof setting.value === "number" ? setting.value : Number(setting.defaultValue);
    const min = typeof setting.min === "number" ? setting.min : 0;
    const max = typeof setting.max === "number" ? setting.max : 100;
    const step = typeof setting.step === "number" ? setting.step : 1;
    return (
      <label className={rowClassName} htmlFor={controlId}>
        {settingHeader}
        <span className="plugin-number-control">
          <input
            id={controlId}
            type="range"
            min={min}
            max={max}
            step={step}
            disabled={disabled}
            value={Number.isFinite(value) ? value : min}
            onChange={(event) => onValueChange(plugin.id, setting, Number(event.currentTarget.value))}
          />
          <output>{Number.isFinite(value) ? value : setting.defaultValue}</output>
        </span>
      </label>
    );
  }

  if (setting.kind === "select") {
    const value = typeof setting.value === "string" ? setting.value : String(setting.defaultValue);
    return (
      <label className={rowClassName} htmlFor={controlId}>
        {settingHeader}
        <select id={controlId} value={value} disabled={disabled} onChange={(event) => onValueChange(plugin.id, setting, event.currentTarget.value)}>
          {setting.options.map((option) => (
            <option key={option.value} value={option.value}>
              {localizedPluginText(option.label, option.labelI18n, locale)}
            </option>
          ))}
        </select>
      </label>
    );
  }

  if (setting.kind === "color") {
    const value = typeof setting.value === "string" ? setting.value : String(setting.defaultValue);
    return (
      <label className={rowClassName} htmlFor={controlId}>
        {settingHeader}
        <input id={controlId} type="color" value={value} disabled={disabled} onChange={(event) => onValueChange(plugin.id, setting, event.currentTarget.value)} />
      </label>
    );
  }

  if (setting.kind === "directory") {
    const value = typeof setting.value === "string" ? setting.value.trim() : "";
    return (
      <div className={rowClassName}>
        {settingHeader}
        <span className="plugin-directory-control">
          <span title={value || t.common.unset}>{value || t.common.unset}</span>
          <button className="settings-reset" type="button" disabled={disabled || isPickerOpen} onClick={() => onChooseDirectory(plugin.id, setting)}>
            {t.common.choose}
          </button>
          <button className="settings-reset" type="button" disabled={disabled || !value} onClick={() => onOpenDirectory(setting)}>
            {t.common.open}
          </button>
        </span>
      </div>
    );
  }

  const value = typeof setting.value === "string" ? setting.value : String(setting.defaultValue);
  if (setting.mpvProperty === "sub-font") {
    const fonts = Array.from(new Set([...systemFontFamilies, value, String(setting.defaultValue)].filter(Boolean))).sort((left, right) =>
      left.localeCompare(right, locale, { sensitivity: "base" }),
    );
    return (
      <label className={rowClassName} htmlFor={controlId}>
        {settingHeader}
        <select id={controlId} value={value} disabled={disabled} onChange={(event) => onValueChange(plugin.id, setting, event.currentTarget.value)}>
          {fonts.map((font) => (
            <option key={font} value={font}>
              {font}
            </option>
          ))}
        </select>
      </label>
    );
  }

  return (
    <label className={rowClassName} htmlFor={controlId}>
      {settingHeader}
      <input id={controlId} type="text" value={value} disabled={disabled} onChange={(event) => onValueChange(plugin.id, setting, event.currentTarget.value)} />
    </label>
  );
}
