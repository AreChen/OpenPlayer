import { Icon } from "../../app/Icon";
import type { PlayerPreferences, ShellPreviewFormatInfo } from "../../app/types";
import type { AppStrings } from "../../i18n";

type PlaybackSettingsPanelProps = {
  t: AppStrings;
  playerPreferences: PlayerPreferences;
  playbackHistoryLength: number;
  shellPreviewFormats: ShellPreviewFormatInfo[];
  selectedShellPreviewFormats: string[];
  shellPreviewRegistrationStatus: string | null;
  isRegisteringShellPreview: boolean;
  onClearPlaybackHistory: () => void;
  onSetIncognitoMode: (enabled: boolean) => void;
  onSetQuietKeyboardControls: (enabled: boolean) => void;
  onToggleAllShellPreviewFormats: () => void;
  onResetShellPreviewFormatsToDefault: () => void;
  onOpenDefaultAppsSettings: () => void;
  onRegisterShellPreviews: () => void;
  onToggleShellPreviewFormat: (extension: string) => void;
};

export function PlaybackSettingsPanel({
  t,
  playerPreferences,
  playbackHistoryLength,
  shellPreviewFormats,
  selectedShellPreviewFormats,
  shellPreviewRegistrationStatus,
  isRegisteringShellPreview,
  onClearPlaybackHistory,
  onSetIncognitoMode,
  onSetQuietKeyboardControls,
  onToggleAllShellPreviewFormats,
  onResetShellPreviewFormatsToDefault,
  onOpenDefaultAppsSettings,
  onRegisterShellPreviews,
  onToggleShellPreviewFormat,
}: PlaybackSettingsPanelProps) {
  const selectedShellPreviewFormatSet = new Set(selectedShellPreviewFormats);
  const allShellPreviewFormatsSelected = shellPreviewFormats.length > 0 && selectedShellPreviewFormats.length === shellPreviewFormats.length;
  const shellPreviewVideoFormats = shellPreviewFormats.filter((format) => format.kind === "video");
  const shellPreviewAudioFormats = shellPreviewFormats.filter((format) => format.kind === "audio");

  return (
    <section className="settings-panel" aria-labelledby="playback-settings-title">
      <div className="settings-panel-heading">
        <div>
          <h3 id="playback-settings-title">{t.settings.playback.title}</h3>
          <span>{t.settings.playback.subtitle}</span>
        </div>
        <button className="settings-reset" type="button" onClick={onClearPlaybackHistory} disabled={!playbackHistoryLength}>
          {t.settings.playback.clearHistory}
        </button>
      </div>

      <div className="preference-list">
        <label className="preference-row">
          <span>
            <strong>{t.settings.playback.incognito}</strong>
            <small>{t.settings.playback.incognitoDescription}</small>
          </span>
          <input type="checkbox" checked={playerPreferences.incognitoMode} onChange={(event) => onSetIncognitoMode(event.currentTarget.checked)} />
          <span className="preference-switch" aria-hidden="true">
            <span />
          </span>
        </label>

        <label className="preference-row">
          <span>
            <strong>{t.settings.playback.quietKeyboard}</strong>
            <small>{t.settings.playback.quietKeyboardDescription}</small>
          </span>
          <input type="checkbox" checked={playerPreferences.quietKeyboardControls} onChange={(event) => onSetQuietKeyboardControls(event.currentTarget.checked)} />
          <span className="preference-switch" aria-hidden="true">
            <span />
          </span>
        </label>
      </div>

      <section className="shell-preview-card" aria-label={t.settings.shellPreview.title}>
        <header className="shell-preview-card-header">
          <span className="shell-preview-card-icon" aria-hidden="true">
            <Icon name="preview" />
          </span>
          <span className="shell-preview-card-copy">
            <strong>{t.settings.shellPreview.title}</strong>
            <small>{t.settings.shellPreview.description}</small>
            {shellPreviewRegistrationStatus && <small className="shell-preview-status">{shellPreviewRegistrationStatus}</small>}
          </span>
          <span className="shell-preview-actions">
            <button className="shell-preview-action" type="button" onClick={onToggleAllShellPreviewFormats} disabled={!shellPreviewFormats.length}>
              {allShellPreviewFormatsSelected ? t.settings.shellPreview.clearAll : t.settings.shellPreview.selectAll}
            </button>
            <button className="shell-preview-action" type="button" onClick={onResetShellPreviewFormatsToDefault} disabled={!shellPreviewFormats.length}>
              {t.settings.shellPreview.defaults}
            </button>
            <button className="shell-preview-action" type="button" onClick={onOpenDefaultAppsSettings}>
              {t.settings.shellPreview.defaultApps}
            </button>
            <button className="shell-preview-action" type="button" onClick={onRegisterShellPreviews} disabled={isRegisteringShellPreview || selectedShellPreviewFormats.length === 0}>
              {isRegisteringShellPreview ? t.settings.shellPreview.registering : t.settings.shellPreview.register(selectedShellPreviewFormats.length)}
            </button>
          </span>
        </header>

        <div className="shell-preview-format-groups">
          {[
            { kind: "video", label: t.settings.shellPreview.video, formats: shellPreviewVideoFormats },
            { kind: "audio", label: t.settings.shellPreview.audio, formats: shellPreviewAudioFormats },
          ].map((group) => (
            <section className="shell-preview-format-group" key={group.kind} aria-label={`${group.label} preview formats`}>
              <header>
                <strong>{group.label}</strong>
                <small>{group.formats.filter((format) => selectedShellPreviewFormatSet.has(format.extension)).length}/{group.formats.length}</small>
              </header>
              <div className="shell-preview-format-grid">
                {group.formats.map((format) => (
                  <button
                    key={format.extension}
                    className={
                      selectedShellPreviewFormatSet.has(format.extension)
                        ? "shell-preview-format shell-preview-format--selected"
                        : "shell-preview-format shell-preview-format--unselected"
                    }
                    type="button"
                    aria-pressed={selectedShellPreviewFormatSet.has(format.extension)}
                    title={format.mime}
                    onClick={() => onToggleShellPreviewFormat(format.extension)}
                  >
                    <span>{format.extension}</span>
                  </button>
                ))}
              </div>
            </section>
          ))}
        </div>
      </section>
    </section>
  );
}
