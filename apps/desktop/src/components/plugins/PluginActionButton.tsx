import { Icon } from "../../app/Icon";
import { localizedPluginText, pluginActionPlacementLabel } from "../../app/pluginRuntime";
import type { PluginActionInstance } from "../../app/types";
import type { AppStrings } from "../../i18n";

type PluginActionButtonProps = {
  instance: PluginActionInstance;
  compact?: boolean;
  locale: string;
  t: AppStrings;
  disabled: boolean;
  onExecute: (instance: PluginActionInstance) => void;
};

export function PluginActionButton({ instance, compact = false, locale, t, disabled, onExecute }: PluginActionButtonProps) {
  const { plugin, action } = instance;
  const actionLabel = localizedPluginText(action.label, action.labelI18n, locale);
  const actionDescription = localizedPluginText(action.description ?? `${plugin.name} · ${pluginActionPlacementLabel(action.placement, t)}`, action.descriptionI18n, locale);

  return (
    <button
      className={compact ? "plugin-action-button plugin-action-button--compact" : "plugin-action-button"}
      type="button"
      title={actionDescription}
      disabled={disabled}
      onClick={() => onExecute(instance)}
    >
      <Icon name={action.icon ?? "plugin"} />
      <span>{actionLabel}</span>
    </button>
  );
}
