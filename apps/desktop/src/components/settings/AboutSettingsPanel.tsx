import { OPENPLAYER_RELEASES_URL, OPENPLAYER_REPOSITORY_URL } from "../../app/constants";
import { canCheckForUpdates, updateActionLabel, updateStatusText } from "../../app/updates";
import type { AppVersionInfo, UpdateState } from "../../app/types";
import type { AppStrings } from "../../i18n";

type AboutSettingsPanelProps = {
  t: AppStrings;
  appVersion: AppVersionInfo | null;
  updateState: UpdateState;
  onCheckForUpdates: () => void;
  onRunUpdateAction: () => void;
  onOpenExternalUrl: (url: string | null | undefined) => void;
};

export function AboutSettingsPanel({ t, appVersion, updateState, onCheckForUpdates, onRunUpdateAction, onOpenExternalUrl }: AboutSettingsPanelProps) {
  const versionInfo = appVersion ?? {
    name: t.common.appName,
    version: t.common.loading,
    license: t.common.loading,
    repository: OPENPLAYER_REPOSITORY_URL,
    releasesUrl: OPENPLAYER_RELEASES_URL,
  };
  const latestVersion = "update" in updateState ? updateState.update.version
    : updateState.status === "current" ? versionInfo.version : t.common.none;
  const updateAction = updateActionLabel(updateState, t);

  return (
    <section className="settings-panel" aria-labelledby="about-settings-title">
      <div className="settings-panel-heading">
        <div>
          <h3 id="about-settings-title">{t.settings.about.title}</h3>
          <span>{t.settings.about.subtitle}</span>
        </div>
      </div>

      <div className="about-panel">
        <section className="about-hero">
          <div className="openplayer-logo about-logo" aria-hidden="true" />
          <div>
            <strong>{versionInfo.name}</strong>
            <p>{t.settings.about.description}</p>
          </div>
        </section>

        <dl className="about-meta">
          <div>
            <dt>{t.settings.about.version}</dt>
            <dd>{versionInfo.version}</dd>
          </div>
          <div>
            <dt>{t.settings.about.license}</dt>
            <dd>{versionInfo.license}</dd>
          </div>
          <div>
            <dt>{t.settings.about.latestVersion}</dt>
            <dd>{latestVersion}</dd>
          </div>
        </dl>

        <section className="about-update-card" aria-label={t.settings.about.update}>
          <div>
            <strong>{t.settings.about.update}</strong>
            <small aria-live="polite">{updateStatusText(updateState, t)}</small>
          </div>
          <div className="about-actions">
            <button className="settings-reset" type="button" onClick={onCheckForUpdates} disabled={!canCheckForUpdates(updateState)}>
              {updateState.status === "checking" ? t.settings.about.checkingShort : t.settings.about.checkForUpdates}
            </button>
            {updateAction && (
              <button className="settings-reset" type="button" onClick={onRunUpdateAction}>
                {updateAction}
              </button>
            )}
          </div>
        </section>

        <div className="about-links">
          <button type="button" onClick={() => onOpenExternalUrl(versionInfo.repository)}>
            {t.settings.about.repository}
          </button>
          <button type="button" onClick={() => onOpenExternalUrl(versionInfo.releasesUrl)}>
            {t.settings.about.releases}
          </button>
        </div>
      </div>
    </section>
  );
}
