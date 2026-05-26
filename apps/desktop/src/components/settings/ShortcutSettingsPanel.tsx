import { Icon } from "../../app/Icon";
import { formatShortcutChord } from "../../app/shortcuts";
import type { ShortcutAction, ShortcutBindings, ShortcutDefinition } from "../../app/types";
import type { AppStrings } from "../../i18n";

type ShortcutSettingsPanelProps = {
  t: AppStrings;
  shortcutDefinitions: ShortcutDefinition[];
  shortcutBindings: ShortcutBindings;
  recordingShortcutAction: ShortcutAction | null;
  onResetShortcutBindings: () => void;
  onStartRecordingShortcut: (action: ShortcutAction) => void;
  onAssignShortcut: (action: ShortcutAction, chord: string | null) => void;
};

export function ShortcutSettingsPanel({
  t,
  shortcutDefinitions,
  shortcutBindings,
  recordingShortcutAction,
  onResetShortcutBindings,
  onStartRecordingShortcut,
  onAssignShortcut,
}: ShortcutSettingsPanelProps) {
  return (
    <section className="settings-panel" aria-labelledby="shortcut-settings-title">
      <div className="settings-panel-heading">
        <div>
          <h3 id="shortcut-settings-title">{t.settings.shortcuts.title}</h3>
          <span>{recordingShortcutAction ? t.common.inputting : t.settings.shortcuts.subtitle}</span>
        </div>
        <button className="settings-reset" type="button" onClick={onResetShortcutBindings}>
          {t.common.restoreDefaults}
        </button>
      </div>

      <div className="shortcut-list">
        {shortcutDefinitions.map((definition) => {
          const isRecording = recordingShortcutAction === definition.action;
          const binding = shortcutBindings[definition.action];

          return (
            <div className="shortcut-row" key={definition.action}>
              <div className="shortcut-meta">
                <span>{definition.label}</span>
                <small>{definition.group}</small>
              </div>
              <div className="shortcut-editor">
                <button
                  className={`shortcut-capture ${isRecording ? "shortcut-capture--recording" : ""}`}
                  type="button"
                  aria-pressed={isRecording}
                  onClick={() => onStartRecordingShortcut(definition.action)}
                >
                  <kbd>{isRecording ? t.common.inputting : formatShortcutChord(binding, t)}</kbd>
                </button>
                <button
                  className="shortcut-clear"
                  type="button"
                  aria-label={`Clear shortcut for ${definition.label}`}
                  disabled={!binding}
                  onClick={() => onAssignShortcut(definition.action, null)}
                >
                  <Icon name="close" />
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
