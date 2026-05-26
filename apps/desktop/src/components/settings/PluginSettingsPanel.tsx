import {
  localizedPluginText,
  pluginActionPlacementLabel,
  pluginCapabilityLabel,
  pluginPackageKindLabel,
} from "../../app/pluginRuntime";
import type { PluginSettingDefinition, PluginSettingValue, ThemePluginSummary } from "../../app/types";
import type { AppStrings } from "../../i18n";
import { PluginSettingControl } from "../plugins/PluginSettingControl";

type PluginSettingsPanelProps = {
  t: AppStrings;
  locale: string;
  plugins: ThemePluginSummary[];
  expandedPluginIds: Set<string>;
  isPickerOpen: boolean;
  systemFontFamilies: string[];
  onImportPluginPackage: () => void;
  onImportPluginDirectory: () => void;
  onImportThemePlugin: () => void;
  onOpenExternalUrl: (url: string | null | undefined) => void;
  onSetPluginEnabled: (pluginId: string, enabled: boolean) => void;
  onTogglePluginSettingsExpanded: (pluginId: string) => void;
  onUninstallPlugin: (pluginId: string) => void;
  onSetPluginSettingValue: (pluginId: string, setting: PluginSettingDefinition, value: PluginSettingValue) => void;
  onChoosePluginDirectory: (pluginId: string, setting: PluginSettingDefinition) => void;
  onOpenPluginDirectory: (setting: PluginSettingDefinition) => void;
};

export function PluginSettingsPanel({
  t,
  locale,
  plugins,
  expandedPluginIds,
  isPickerOpen,
  systemFontFamilies,
  onImportPluginPackage,
  onImportPluginDirectory,
  onImportThemePlugin,
  onOpenExternalUrl,
  onSetPluginEnabled,
  onTogglePluginSettingsExpanded,
  onUninstallPlugin,
  onSetPluginSettingValue,
  onChoosePluginDirectory,
  onOpenPluginDirectory,
}: PluginSettingsPanelProps) {
  return (
    <section className="settings-panel" aria-labelledby="plugin-settings-title">
      <div className="settings-panel-heading">
        <div>
          <h3 id="plugin-settings-title">{t.settings.plugins.title}</h3>
          <span>{t.settings.plugins.subtitle}</span>
        </div>
        <div className="settings-heading-actions">
          <button className="settings-reset" type="button" onClick={onImportPluginPackage} disabled={isPickerOpen}>
            {t.settings.plugins.installPackage}
          </button>
          <button className="settings-reset" type="button" onClick={onImportPluginDirectory} disabled={isPickerOpen}>
            {t.settings.plugins.importDirectory}
          </button>
          <button className="settings-reset" type="button" onClick={onImportThemePlugin} disabled={isPickerOpen}>
            {t.settings.plugins.importJson}
          </button>
        </div>
      </div>

      <div className="plugin-list">
        {plugins.map((plugin) => {
          const isExpanded = expandedPluginIds.has(plugin.id);
          const pluginDescription = plugin.description;
          return (
            <div className={isExpanded ? "plugin-row plugin-row--expanded" : "plugin-row"} key={plugin.id}>
              <div className="plugin-row-header">
                <div className="plugin-meta">
                  <span>{plugin.name}</span>
                  <small>
                    {pluginPackageKindLabel(plugin.packageKind, t)} · {t.settings.plugins.runtime(plugin.runtime)} · v{plugin.version}
                  </small>
                </div>
                <div className="plugin-row-actions">
                  {plugin.updateUrl && (
                    <button className="settings-reset plugin-update-link" type="button" onClick={() => onOpenExternalUrl(plugin.updateUrl)}>
                      {t.settings.plugins.update}
                    </button>
                  )}
                  <label className="plugin-toggle">
                    <input type="checkbox" checked={plugin.enabled} onChange={(event) => onSetPluginEnabled(plugin.id, event.currentTarget.checked)} />
                    <span>{plugin.enabled ? t.settings.plugins.enabled : t.settings.plugins.disabled}</span>
                  </label>
                  {plugin.settings.length > 0 && (
                    <button className="settings-reset plugin-settings-toggle" type="button" aria-expanded={isExpanded} onClick={() => onTogglePluginSettingsExpanded(plugin.id)}>
                      {t.settings.plugins.settings}
                    </button>
                  )}
                  <button className="settings-reset plugin-uninstall" type="button" onClick={() => onUninstallPlugin(plugin.id)}>
                    {t.settings.plugins.uninstall}
                  </button>
                </div>
              </div>
              <div className="plugin-detail-grid">
                {[
                  t.settings.plugins.themeCount(plugin.themeCount),
                  t.settings.plugins.capabilityCount(plugin.capabilityCount),
                  t.settings.plugins.settingCount(plugin.settingCount),
                  t.settings.plugins.actionCount(plugin.actionCount),
                  t.settings.plugins.apiVersion(plugin.apiVersion),
                  plugin.minHostVersion ? t.settings.plugins.minHostVersion(plugin.minHostVersion) : null,
                  plugin.author ? t.settings.plugins.author(plugin.author) : null,
                ]
                  .filter((label): label is string => Boolean(label))
                  .map((label) => (
                    <span key={label} title={label}>
                      {label}
                    </span>
                  ))}
              </div>
              {pluginDescription && <p className="plugin-description">{pluginDescription}</p>}
              {plugin.installPath && (
                <div className="plugin-install-path">
                  <span>{t.settings.plugins.installPath}</span>
                  <code title={plugin.installPath}>{plugin.installPath}</code>
                </div>
              )}
              {plugin.capabilities.length > 0 && (
                <div className="plugin-chip-row" aria-label={t.settings.plugins.capabilities}>
                  {plugin.capabilities.map((capability) => (
                    <span className="plugin-chip" key={capability.id} title={localizedPluginText(capability.description ?? capability.kind, capability.descriptionI18n, locale)}>
                      {localizedPluginText(pluginCapabilityLabel(capability.kind, t), capability.nameI18n, locale)}
                    </span>
                  ))}
                </div>
              )}
              {(plugin.permissions.length > 0 || plugin.actions.length > 0) && (
                <div className="plugin-technical-summary">
                  {plugin.permissions.length > 0 && <span>{t.settings.plugins.permissions}: {plugin.permissions.join(", ")}</span>}
                  {plugin.actions.length > 0 && (
                    <span>
                      {t.settings.plugins.actions}:{" "}
                      {plugin.actions
                        .map((action) => `${localizedPluginText(action.label, action.labelI18n, locale)} (${pluginActionPlacementLabel(action.placement, t)})`)
                        .join(", ")}
                    </span>
                  )}
                </div>
              )}
              {plugin.settings.length > 0 && isExpanded && (
                <div className="plugin-setting-list">
                  {plugin.settings.map((setting) => (
                    <PluginSettingControl
                      key={`${plugin.id}:${setting.id}`}
                      plugin={plugin}
                      setting={setting}
                      locale={locale}
                      t={t}
                      isPickerOpen={isPickerOpen}
                      systemFontFamilies={systemFontFamilies}
                      onValueChange={onSetPluginSettingValue}
                      onChooseDirectory={onChoosePluginDirectory}
                      onOpenDirectory={onOpenPluginDirectory}
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
        {!plugins.length && <div className="plugin-empty">{t.settings.plugins.empty}</div>}
      </div>
    </section>
  );
}
