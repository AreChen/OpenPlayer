import { Icon } from "../app/Icon";
import { formatShortcutChord } from "../app/shortcuts";
import type { ContextMenuPosition, IconName } from "../app/types";
import type { AppStrings } from "../i18n";

export type ContextMenuEntry =
  | { type: "item"; id: string; label: string; icon: IconName; shortcut?: string | null; disabled?: boolean; onSelect: () => void }
  | { type: "separator"; id: string };

type ContextMenuProps = {
  t: AppStrings;
  position: ContextMenuPosition;
  items: ContextMenuEntry[];
  onClose: () => void;
};

export function ContextMenu({ t, position, items, onClose }: ContextMenuProps) {
  return (
    <div
      className="context-menu"
      role="menu"
      aria-label={t.contextMenu.settings}
      style={{ left: position.x, top: position.y }}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
      }}
      onPointerDown={(event) => event.stopPropagation()}
    >
      {items.map((item) =>
        item.type === "separator" ? (
          <div key={item.id} className="context-menu-separator" role="separator" />
        ) : (
          <button
            key={item.id}
            className="context-menu-item"
            type="button"
            role="menuitem"
            disabled={item.disabled}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
          >
            <Icon name={item.icon} />
            <span>{item.label}</span>
            {item.shortcut && <kbd>{formatShortcutChord(item.shortcut, t)}</kbd>}
          </button>
        ),
      )}
    </div>
  );
}
