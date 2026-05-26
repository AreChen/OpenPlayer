import { OPENPLAYER_RELEASES_URL, openPlayerLogoUrl } from "../../app/constants";
import { updateStatusText } from "../../app/updates";
import type { AppVersionInfo, UpdateState } from "../../app/types";
import type { AppStrings } from "../../i18n";

type AboutSettingsPanelProps = {
  t: AppStrings;
  appVersion: AppVersionInfo | null;
  updateState: UpdateState;
  onCheckForUpdates: () => void;
  onOpenUpdateDownload: () => void;
  onOpenExternalUrl: (url: string | null | undefined) => void;
};

export function AboutSettingsPanel({ t, appVersion, updateState, onCheckForUpdates, onOpenUpdateDownload, onOpenExternalUrl }: AboutSettingsPanelProps) {
  const versionInfo = appVersion ?? {
    name: t.common.appName,
    version: t.common.loading,
    license: t.common.loading,
    repository: OPENPLAYER_RELEASES_URL.replace(/\/releases\/latest$/, ""),
    releasesUrl: OPENPLAYER_RELEASES_URL,
  };
  const latestVersion = updateState.latest?.version ?? t.common.none;
  const downloadLabel = updateState.status === "available" && updateState.asset ? t.settings.about.downloadUpdate : t.settings.about.openReleasePage;

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
          <img src={openPlayerLogoUrl} alt="" draggable={false} />
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
            <small>{updateStatusText(updateState, t)}</small>
          </div>
          <div className="about-actions">
            <button className="settings-reset" type="button" onClick={onCheckForUpdates} disabled={updateState.status === "checking"}>
              {updateState.status === "checking" ? t.settings.about.checkingShort : t.settings.about.checkForUpdates}
            </button>
            <button className="settings-reset" type="button" onClick={onOpenUpdateDownload} disabled={updateState.status !== "available" && !updateState.latest}>
              {downloadLabel}
            </button>
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
