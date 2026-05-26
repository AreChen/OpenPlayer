import { Icon } from "../../app/Icon";
import type { SettingsSection } from "../../app/types";
import type { AppStrings } from "../../i18n";

type SettingsNavProps = {
  t: AppStrings;
  settingsSection: SettingsSection;
  onSectionChange: (section: SettingsSection) => void;
};

export function SettingsNav({ t, settingsSection, onSectionChange }: SettingsNavProps) {
  return (
    <nav className="settings-nav" aria-label={t.settings.title}>
      <SettingsNavItem
        active={settingsSection === "appearance"}
        icon="palette"
        label={t.settings.nav.appearance}
        section="appearance"
        onSectionChange={onSectionChange}
      />
      <SettingsNavItem
        active={settingsSection === "plugins"}
        icon="plugin"
        label={t.settings.nav.plugins}
        section="plugins"
        onSectionChange={onSectionChange}
      />
      <SettingsNavItem
        active={settingsSection === "playback"}
        icon="play"
        label={t.settings.nav.playback}
        section="playback"
        onSectionChange={onSectionChange}
      />
      <SettingsNavItem
        active={settingsSection === "shortcuts"}
        icon="settings"
        label={t.settings.nav.shortcuts}
        section="shortcuts"
        onSectionChange={onSectionChange}
      />
      <SettingsNavItem
        active={settingsSection === "about"}
        icon="info"
        label={t.settings.nav.about}
        section="about"
        onSectionChange={onSectionChange}
      />
    </nav>
  );
}

type SettingsNavItemProps = {
  active: boolean;
  icon: "palette" | "plugin" | "play" | "settings" | "info";
  label: string;
  section: SettingsSection;
  onSectionChange: (section: SettingsSection) => void;
};

function SettingsNavItem({ active, icon, label, section, onSectionChange }: SettingsNavItemProps) {
  return (
    <button
      className={`settings-nav-item ${active ? "settings-nav-item--active" : ""}`}
      type="button"
      aria-current={active ? "page" : undefined}
      onClick={() => onSectionChange(section)}
    >
      <Icon name={icon} />
      <span>{label}</span>
    </button>
  );
}
