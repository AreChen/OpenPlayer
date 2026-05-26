import { Icon } from "../../app/Icon";
import { SettingsDialogContent } from "./SettingsDialogContent";
import type { SettingsDialogProps } from "./SettingsDialog.types";
import { SettingsNav } from "./SettingsNav";

export function SettingsDialog({
  t,
  dialogRef,
  settingsSection,
  onSectionChange,
  onClose,
  ...contentProps
}: SettingsDialogProps) {
  return (
    <div
      className="settings-backdrop"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <section
        ref={dialogRef}
        className="settings-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        tabIndex={-1}
        onContextMenu={(event) => event.stopPropagation()}
        onPointerDown={(event) => event.stopPropagation()}
      >
        <header className="settings-header">
          <div>
            <span className="settings-kicker">OpenPlayer</span>
            <h2 id="settings-title">{t.settings.title}</h2>
          </div>
          <button className="settings-close" type="button" aria-label={t.controls.close} onClick={onClose}>
            <Icon name="close" />
          </button>
        </header>

        <div className="settings-layout">
          <SettingsNav t={t} settingsSection={settingsSection} onSectionChange={onSectionChange} />
          <SettingsDialogContent t={t} settingsSection={settingsSection} {...contentProps} />
        </div>
      </section>
    </div>
  );
}
